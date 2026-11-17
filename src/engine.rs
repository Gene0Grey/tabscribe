//! Everything that happens after "transcribe": fetch audio with yt-dlp, decode it,
//! run whisper locally, keep the result on disk.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

pub type Tx = UnboundedSender<Ev>;

const STOPPED: &str = "stopped";
#[cfg(windows)]
const YTDLP: (&str, &str) = ("yt-dlp.exe", "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe");
#[cfg(not(windows))]
const YTDLP: (&str, &str) = ("yt-dlp", "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos");

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Model {
    Fast,
    Balanced,
    Best,
}

impl Model {
    pub fn label(self) -> &'static str {
        match self {
            Model::Fast => "fast",
            Model::Balanced => "balanced",
            Model::Best => "accurate",
        }
    }
    pub fn detail(self) -> &'static str {
        match self {
            Model::Fast => "whisper base · 147 MB",
            Model::Balanced => "whisper small · 487 MB",
            Model::Best => "whisper large turbo · 574 MB",
        }
    }
    fn file(self) -> &'static str {
        match self {
            Model::Fast => "ggml-base.bin",
            Model::Balanced => "ggml-small.bin",
            Model::Best => "ggml-large-v3-turbo-q5_0.bin",
        }
    }
    pub fn size(self) -> &'static str {
        self.detail().rsplit(" · ").next().unwrap_or_default()
    }
    pub fn next(self) -> Self {
        match self {
            Model::Fast => Model::Balanced,
            Model::Balanced => Model::Best,
            Model::Best => Model::Fast,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Seg {
    pub t0: i64, // ms
    pub t1: i64,
    pub text: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Transcript {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub duration: f64, // seconds
    pub model: Model,
    pub created: u64, // unix seconds
    pub segs: Vec<Seg>,
}

impl Transcript {
    pub fn new(url: &str, title: Option<String>, model: Model) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        Self {
            id: now.as_millis() as u64,
            title: title.unwrap_or_else(|| host(url)),
            url: url.to_string(),
            duration: 0.0,
            model,
            created: now.as_secs(),
            segs: vec![],
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Stage {
    Setup { what: &'static str, pct: Option<f32>, mb: u32 },
    Updating,
    Fetching(Option<f32>),
    Reading,
    Listening(f32),
    Live { secs: f32, level: f32 },
    Finishing,
}

pub enum Ev {
    Stage(Stage),
    Title(String, f64),
    Seg(Seg),
    Done(Transcript),
    Failed(String),
    Stopped,
}

// ---------- the job ----------

pub fn run(mut t: Transcript, tx: Tx, cancel: Arc<AtomicBool>) {
    let r = (|| -> Result<Transcript, String> {
        let yt = ytdlp(&tx, &cancel)?;
        let model = ensure_model(t.model, &tx, &cancel)?;
        let _ = tx.send(Ev::Stage(Stage::Fetching(None)));
        let (file, title, dur) = fetch(&yt, &t.url, &tx, &cancel)?;
        if let Some(title) = title {
            t.title = title;
        }
        t.duration = dur;
        let _ = tx.send(Ev::Stage(Stage::Reading));
        // ponytail: whole track in RAM as 16 kHz mono f32 (~230 MB per hour); chunk it if 4 h+ videos matter.
        let audio = decode(&file)?;
        if t.duration <= 0.0 {
            t.duration = audio.len() as f64 / 16_000.0;
        }
        let _ = tx.send(Ev::Stage(Stage::Listening(0.0)));
        t.segs = transcribe(&audio, &model, &tx, &cancel)?;
        let _ = fs::remove_dir_all(data().join("tmp"));
        let _ = fs::write(dir("library").join(format!("{}.json", t.id)), serde_json::to_vec(&t).unwrap_or_default());
        Ok(t)
    })();
    let _ = tx.send(match r {
        _ if cancel.load(Relaxed) => Ev::Stopped,
        Ok(t) => Ev::Done(t),
        Err(e) => Ev::Failed(e),
    });
}

// ---------- storage ----------

fn data() -> PathBuf {
    let d = directories::ProjectDirs::from("", "", "tabscribe")
        .map(|p| p.data_local_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("tabscribe"));
    let _ = fs::create_dir_all(&d);
    d
}

fn dir(name: &str) -> PathBuf {
    let d = data().join(name);
    let _ = fs::create_dir_all(&d);
    d
}

pub fn load_library() -> Vec<Transcript> {
    let mut v: Vec<Transcript> = fs::read_dir(dir("library"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| serde_json::from_slice(&fs::read(e.path()).ok()?).ok())
        .collect();
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v
}

pub fn load_model() -> Model {
    fs::read(data().join("model.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Model::Balanced)
}

pub fn save_model(m: Model) {
    let _ = fs::write(data().join("model.json"), serde_json::to_vec(&m).unwrap_or_default());
}

pub fn model_ready(m: Model) -> bool {
    dir("models").join(m.file()).exists()
}

// ---------- downloads ----------

fn download(url: &str, dest: &Path, what: &'static str, tx: &Tx, cancel: &AtomicBool) -> Result<(), String> {
    let offline = || format!("Couldn't download the {what}. Check your internet connection and try again.");
    let _ = tx.send(Ev::Stage(Stage::Setup { what, pct: None, mb: 0 }));
    let client = reqwest::blocking::Client::builder()
        .timeout(None::<Duration>)
        .user_agent("tabscribe")
        .build()
        .map_err(|_| offline())?;
    let mut res = client.get(url).send().and_then(|r| r.error_for_status()).map_err(|_| offline())?;
    let total = res.content_length().unwrap_or(0);
    let mb = (total / 1_000_000) as u32;
    let _ = tx.send(Ev::Stage(Stage::Setup { what, pct: Some(0.0), mb }));
    let part = dest.with_extension("part");
    let mut f = File::create(&part).map_err(|e| e.to_string())?;
    let (mut buf, mut got, mut last) = (vec![0u8; 1 << 16], 0u64, u64::MAX);
    loop {
        if cancel.load(Relaxed) {
            drop(f);
            let _ = fs::remove_file(&part);
            return Err(STOPPED.into());
        }
        let n = res.read(&mut buf).map_err(|_| offline())?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| format!("Couldn't save the {what}: {e}"))?;
        got += n as u64;
        if total > 0 && got * 1000 / total != last {
            last = got * 1000 / total;
            let _ = tx.send(Ev::Stage(Stage::Setup { what, pct: Some(last as f32 / 10.0), mb }));
        }
    }
    drop(f);
    if total > 0 && got != total {
        let _ = fs::remove_file(&part);
        return Err(offline());
    }
    fs::rename(&part, dest).map_err(|e| e.to_string())
}

fn ytdlp(tx: &Tx, cancel: &AtomicBool) -> Result<PathBuf, String> {
    let p = dir("bin").join(YTDLP.0);
    if !p.exists() {
        download(YTDLP.1, &p, "video fetcher", tx, cancel)?;
    } else if p.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()) > Some(Duration::from_secs(3 * 86_400)) {
        // Sites change constantly; a stale yt-dlp is the most common failure.
        let _ = tx.send(Ev::Stage(Stage::Updating));
        let _ = hidden(&p).arg("-U").output();
        let _ = File::options().write(true).open(&p).and_then(|f| f.set_modified(SystemTime::now()));
    }
    Ok(p)
}

fn ensure_model(m: Model, tx: &Tx, cancel: &AtomicBool) -> Result<PathBuf, String> {
    let p = dir("models").join(m.file());
    if !p.exists() {
        let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", m.file());
        download(&url, &p, "speech model", tx, cancel)?;
    }
    Ok(p)
}

// ---------- fetch audio ----------

fn hidden(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut c = Command::new(program);
    c.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flash
    }
    c
}

fn fetch(yt: &Path, url: &str, tx: &Tx, cancel: &AtomicBool) -> Result<(PathBuf, Option<String>, f64), String> {
    let tmp = data().join("tmp");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut child = hidden(yt)
        .env("PYTHONIOENCODING", "utf-8")
        .args(["--no-playlist", "--no-simulate", "--newline", "--progress", "--encoding", "utf-8"])
        .args(["-f", "ba[ext=m4a]/ba[acodec^=mp4a]/ba/b"])
        .args(["--progress-template", "download:PROG %(progress._percent_str)s"])
        .args(["--print", "before_dl:DUR %(duration)s", "--print", "before_dl:TITLE %(title)s"])
        .args(["--print", "after_move:FILE %(filepath)s"])
        .arg("-o")
        .arg(tmp.join("audio.%(ext)s"))
        .arg("--")
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Couldn't start the video fetcher.".to_string())?;

    let mut stderr = child.stderr.take().expect("piped");
    let err = std::thread::spawn(move || {
        let mut s = Vec::new();
        let _ = stderr.read_to_end(&mut s);
        String::from_utf8_lossy(&s).into_owned()
    });

    let (mut file, mut title, mut dur) = (None, None, 0.0);
    let mut out = BufReader::new(child.stdout.take().expect("piped"));
    let mut raw = Vec::new();
    while out.read_until(b'\n', &mut raw).unwrap_or(0) > 0 {
        let line = String::from_utf8_lossy(&raw).trim().to_string();
        raw.clear();
        if cancel.load(Relaxed) {
            let _ = child.kill();
            break;
        }
        if let Some(p) = line.strip_prefix("PROG ") {
            let p: String = p.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(p) = p.parse::<f32>() {
                let _ = tx.send(Ev::Stage(Stage::Fetching(Some(p))));
            }
        } else if let Some(d) = line.strip_prefix("DUR ") {
            dur = d.parse().unwrap_or(0.0);
        } else if let Some(t) = line.strip_prefix("TITLE ") {
            title = Some(t.to_string());
            let _ = tx.send(Ev::Title(t.to_string(), dur));
        } else if let Some(f) = line.strip_prefix("FILE ") {
            file = Some(PathBuf::from(f));
        }
    }
    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    let err = err.join().unwrap_or_default();
    if cancel.load(Relaxed) {
        return Err(STOPPED.into());
    }
    match file {
        Some(f) if ok && f.exists() => Ok((f, title, dur)),
        _ => Err(explain(&err)),
    }
}

/// Turn yt-dlp's stderr into something a non-technical person can act on.
fn explain(err: &str) -> String {
    let low = err.to_lowercase();
    let last = err.lines().rev().find(|l| l.contains("ERROR")).unwrap_or("");
    let last = last.split_once("ERROR:").map_or(last, |(_, r)| r).trim();
    if low.contains("unsupported url") {
        "This page doesn't have a video we can fetch. Open the video itself, then try again.".into()
    } else if low.contains("drm") {
        "This video is copy-protected, so its audio can't be fetched.".into()
    } else if low.contains("private video") {
        "This video is private.".into()
    } else if low.contains("sign in") || low.contains("log in") || low.contains("cookies") {
        "This video needs a signed-in account, so it can't be fetched.".into()
    } else if low.contains("getaddrinfo") || low.contains("timed out") || low.contains("connection") {
        "Couldn't reach the site. Check your internet connection and try again.".into()
    } else if last.is_empty() {
        "Couldn't fetch this video's audio.".into()
    } else {
        format!("Couldn't fetch this video's audio. {last}")
    }
}

// ---------- decode to 16 kHz mono ----------

fn decode(path: &Path) -> Result<Vec<f32>, String> {
    decode_native(path).or_else(|e| decode_ffmpeg(path).ok_or(e))
}

fn decode_native(path: &Path) -> Result<Vec<f32>, String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
    use symphonia::core::errors::Error;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let unsupported = |_| "This video's audio format isn't supported yet.".to_string();
    let mss = MediaSourceStream::new(Box::new(File::open(path).map_err(|e| e.to_string())?), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(unsupported)?
        .format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("This video has no sound to transcribe.")?;
    let (id, rate) = (track.id, track.codec_params.sample_rate.unwrap_or(16_000));
    let mut dec = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(unsupported)?;
    let mut out = Mono16k::new(rate)?;
    loop {
        let pkt = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(Error::ResetRequired) => break,
            Err(e) => return Err(format!("Couldn't read the audio: {e}")),
        };
        if pkt.track_id() != id {
            continue;
        }
        let buf = match dec.decode(&pkt) {
            Ok(b) => b,
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Couldn't read the audio: {e}")),
        };
        let spec = *buf.spec();
        let ch = spec.channels.count().max(1);
        let mut sb = SampleBuffer::<f32>::new(buf.capacity() as u64, spec);
        sb.copy_interleaved_ref(buf);
        out.push(sb.samples().chunks(ch).map(|f| f.iter().sum::<f32>() / ch as f32));
    }
    Ok(out.finish())
}

/// Fallback for codecs symphonia lacks (e.g. Opus), only if ffmpeg happens to be installed.
fn decode_ffmpeg(path: &Path) -> Option<Vec<f32>> {
    let out = hidden("ffmpeg")
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(path)
        .args(["-ac", "1", "-ar", "16000", "-f", "f32le", "-"])
        .output()
        .ok()?;
    out.status.success().then(|| {
        out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
    })
}

struct Mono16k {
    rs: Option<rubato::FftFixedIn<f32>>,
    pending: Vec<f32>,
    out: Vec<f32>,
}

impl Mono16k {
    fn new(rate: u32) -> Result<Self, String> {
        let rs = if rate == 16_000 {
            None
        } else {
            Some(rubato::FftFixedIn::new(rate as usize, 16_000, 1024, 2, 1).map_err(|e| e.to_string())?)
        };
        Ok(Self { rs, pending: vec![], out: vec![] })
    }

    fn push(&mut self, mono: impl Iterator<Item = f32>) {
        use rubato::Resampler;
        let Some(rs) = &mut self.rs else {
            self.out.extend(mono);
            return;
        };
        self.pending.extend(mono);
        let mut at = 0;
        while self.pending.len() - at >= rs.input_frames_next() {
            let n = rs.input_frames_next();
            if let Ok(o) = rs.process(&[&self.pending[at..at + n]], None) {
                self.out.extend_from_slice(&o[0]);
            }
            at += n;
        }
        self.pending.drain(..at);
    }

    fn finish(mut self) -> Vec<f32> {
        use rubato::Resampler;
        if let Some(rs) = &mut self.rs {
            if !self.pending.is_empty() {
                if let Ok(o) = rs.process_partial(Some(&[&self.pending[..]]), None) {
                    self.out.extend_from_slice(&o[0]);
                }
            }
        }
        self.out
    }
}

// ---------- whisper ----------

fn load_whisper(model: &Path) -> Result<whisper_rs::WhisperState, String> {
    use whisper_rs::{WhisperContext, WhisperContextParameters};
    let ctx = WhisperContext::new_with_params(model, WhisperContextParameters::default()).map_err(|_| {
        let _ = fs::remove_file(model); // damaged download: fetch it fresh next time
        "The speech model couldn't be loaded. Press try again to download it fresh.".to_string()
    })?;
    ctx.create_state().map_err(|e| e.to_string())
}

fn transcribe(audio: &[f32], model: &Path, tx: &Tx, cancel: &Arc<AtomicBool>) -> Result<Vec<Seg>, String> {
    let mut st = load_whisper(model)?;
    whisper(&mut st, audio, 0, tx, Some(cancel))
}

/// Run whisper over `audio`, shifting timestamps by `offset_ms`. With `cancel` (whole-file mode)
/// lines stream out as they are found and progress is reported; without it the caller sends them.
fn whisper(st: &mut whisper_rs::WhisperState, audio: &[f32], offset_ms: i64, tx: &Tx, cancel: Option<&Arc<AtomicBool>>) -> Result<Vec<Seg>, String> {
    use whisper_rs::{FullParams, SamplingStrategy, SegmentCallbackData};

    let mut p = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    p.set_language(Some("auto"));
    p.set_n_threads(std::thread::available_parallelism().map_or(4, |n| n.get().min(8)) as i32);
    p.set_print_progress(false);
    p.set_print_realtime(false);
    p.set_print_special(false);
    p.set_print_timestamps(false);
    if let Some(cancel) = cancel {
        let t1 = tx.clone();
        p.set_segment_callback_safe(move |s: SegmentCallbackData| {
            let s = seg(s.start_timestamp, s.end_timestamp, &s.text, offset_ms);
            if !s.text.is_empty() {
                let _ = t1.send(Ev::Seg(s));
            }
        });
        let t2 = tx.clone();
        p.set_progress_callback_safe(move |pct: i32| {
            let _ = t2.send(Ev::Stage(Stage::Listening(pct as f32)));
        });
        // whisper-rs 0.16's set_abort_callback_safe casts its boxed closure to the wrong type and aborts
        // every run, so hand whisper.cpp the flag directly. `cancel` outlives st.full() below.
        unsafe extern "C" fn should_abort(flag: *mut std::ffi::c_void) -> bool {
            unsafe { (*(flag as *const AtomicBool)).load(Relaxed) }
        }
        unsafe {
            p.set_abort_callback(Some(should_abort));
            p.set_abort_callback_user_data(Arc::as_ptr(cancel) as *mut std::ffi::c_void);
        }
    }
    let started = std::time::Instant::now();
    st.full(p, audio)
        .map_err(|e| format!("Transcription stopped unexpectedly ({e:?}). Try again, or pick the fast model."))?;
    if cfg!(test) {
        println!("{}\nwhisper took {:.1}s", whisper_rs::print_system_info(), started.elapsed().as_secs_f32());
    }
    Ok((0..st.full_n_segments())
        .filter_map(|i| st.get_segment(i))
        .map(|s| seg(s.start_timestamp(), s.end_timestamp(), &s.to_str_lossy().unwrap_or_default(), offset_ms))
        .filter(|s| !s.text.is_empty())
        .collect())
}

fn seg(t0: i64, t1: i64, text: &str, offset_ms: i64) -> Seg {
    // whisper reports centiseconds
    Seg { t0: t0 * 10 + offset_ms, t1: t1 * 10 + offset_ms, text: text.trim().to_string() }
}

// ---------- live: listen to the browser while it plays ----------

/// Public video sites yt-dlp can download directly (faster than playing them).
/// Everything else, class portals included, is transcribed live from the browser's sound.
pub fn fetchable(url: &str) -> bool {
    let h = host(url);
    ["youtube.com", "youtu.be", "vimeo.com", "dailymotion.com", "ted.com", "soundcloud.com", "twitch.tv"]
        .iter()
        .any(|d| h == *d || h.ends_with(&format!(".{d}")))
}

const CHUNK: usize = 25 * 16_000; // whisper is at its best on windows up to 30 s

pub fn listen(mut t: Transcript, pid: u32, tx: Tx, finish: Arc<AtomicBool>) {
    let r = (|| -> Result<Transcript, String> {
        let model = ensure_model(t.model, &tx, &finish)?;
        let mut st = load_whisper(&model)?;
        let buf = Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
        let level = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let cap = {
            let (buf, level, finish) = (buf.clone(), level.clone(), finish.clone());
            std::thread::spawn(move || capture(pid, buf, level, finish))
        };
        let mut done: usize = 0; // samples already transcribed
        loop {
            std::thread::sleep(Duration::from_millis(400));
            let finishing = finish.load(Relaxed) || cap.is_finished();
            let have = buf.lock().map(|b| b.len()).unwrap_or(0);
            let secs = (done + have) as f32 / 16_000.0;
            let _ = tx.send(Ev::Stage(if finishing {
                Stage::Finishing
            } else {
                Stage::Live { secs, level: f32::from_bits(level.load(Relaxed)) }
            }));
            if have < CHUNK && !finishing {
                continue;
            }
            let chunk: Vec<f32> = {
                let mut b = buf.lock().map_err(|e| e.to_string())?;
                let cut = if finishing { b.len() } else { quiet_cut(&b[..CHUNK]) };
                b.drain(..cut).collect()
            };
            if rms(&chunk) > 0.003 {
                for s in whisper(&mut st, &chunk, done as i64 * 1000 / 16_000, &tx, None)? {
                    let _ = tx.send(Ev::Seg(s.clone()));
                    t.segs.push(s);
                }
            }
            done += chunk.len();
            if finishing {
                break;
            }
        }
        let captured = cap.join().unwrap_or_else(|_| Err("The audio capture crashed.".into()));
        t.duration = done as f64 / 16_000.0;
        if t.segs.is_empty() {
            return Err(captured.err().unwrap_or_else(|| {
                "Nothing was heard. Start the video in your browser with its sound on, then try again.".into()
            }));
        }
        let _ = fs::write(dir("library").join(format!("{}.json", t.id)), serde_json::to_vec(&t).unwrap_or_default());
        Ok(t)
    })();
    let _ = tx.send(match r {
        Ok(t) => Ev::Done(t),
        Err(e) if e == STOPPED => Ev::Stopped,
        Err(e) => Ev::Failed(e),
    });
}

/// Cut a window at its quietest 100 ms in the last 40%, so words are not split between chunks.
fn quiet_cut(w: &[f32]) -> usize {
    const F: usize = 1_600;
    let from = w.len() * 6 / 10 / F;
    (from..w.len() / F)
        .min_by(|&a, &b| rms(&w[a * F..(a + 1) * F]).total_cmp(&rms(&w[b * F..(b + 1) * F])))
        .map_or(w.len(), |i| i * F + F / 2)
}

fn rms(x: &[f32]) -> f32 {
    if x.is_empty() { 0.0 } else { (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt() }
}

/// Capture everything the browser process tree plays (Windows 10 2004+ process loopback).
#[cfg(windows)]
fn capture(pid: u32, buf: Arc<std::sync::Mutex<Vec<f32>>>, level: Arc<std::sync::atomic::AtomicU32>, finish: Arc<AtomicBool>) -> Result<(), String> {
    use std::collections::VecDeque;
    use wasapi::{AudioClient, Direction, SampleType, StreamMode, WaveFormat, initialize_mta};

    let err = |e: wasapi::WasapiError| format!("Couldn't listen to the browser's sound ({e}).");
    let _ = initialize_mta();
    let fmt = WaveFormat::new(32, 32, SampleType::Float, 48_000, 2, None);
    let mut client = AudioClient::new_application_loopback_client(pid, true).map_err(err)?;
    client
        .initialize_client(&fmt, Direction::Capture, StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: 0 }, None)
        .map_err(err)?;
    let event = client.set_get_eventhandle().map_err(err)?;
    let cap = client.get_audiocaptureclient().map_err(err)?;
    client.start_stream().map_err(err)?;
    let (mut q, mut rs) = (VecDeque::<u8>::new(), Mono16k::new(48_000)?);
    while !finish.load(Relaxed) {
        let _ = event.wait_for_event(250); // times out while nothing plays; that is fine
        while cap.get_next_packet_size().map_err(err)?.unwrap_or(0) > 0 {
            cap.read_from_device_to_deque(&mut q).map_err(err)?;
        }
        let whole = q.len() - q.len() % 8;
        if whole == 0 {
            continue;
        }
        let bytes: Vec<u8> = q.drain(..whole).collect();
        let mono: Vec<f32> = bytes
            .chunks_exact(8)
            .map(|f| (f32::from_le_bytes([f[0], f[1], f[2], f[3]]) + f32::from_le_bytes([f[4], f[5], f[6], f[7]])) / 2.0)
            .collect();
        level.store(mono.iter().fold(0f32, |m, s| m.max(s.abs())).to_bits(), Relaxed);
        rs.push(mono.into_iter());
        if let Ok(mut b) = buf.lock() {
            b.append(&mut rs.out);
        }
    }
    let _ = client.stop_stream();
    Ok(())
}

#[cfg(not(windows))]
fn capture(_: u32, _: Arc<std::sync::Mutex<Vec<f32>>>, _: Arc<std::sync::atomic::AtomicU32>, _: Arc<AtomicBool>) -> Result<(), String> {
    Err("Live listening isn't available on this system yet.".into()) // ponytail: ScreenCaptureKit on macOS
}

// ---------- formatting & export ----------

pub fn clock(ms: i64) -> String {
    let s = ms / 1000;
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
}

fn srt_time(ms: i64) -> String {
    format!("{:02}:{:02}:{:02},{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
}

pub fn to_plain(t: &Transcript) -> String {
    t.segs.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("\n")
}

pub fn to_txt(t: &Transcript) -> String {
    let body: Vec<String> = t.segs.iter().map(|s| format!("[{}] {}", clock(s.t0), s.text)).collect();
    format!("{}\n{}\n\n{}\n", t.title, t.url, body.join("\n"))
}

pub fn to_srt(t: &Transcript) -> String {
    t.segs
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{}\n{} --> {}\n{}\n", i + 1, srt_time(s.t0), srt_time(s.t1), s.text))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let h = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    h.trim_start_matches("www.").trim_start_matches("m.").to_string()
}

pub fn jump_url(url: &str, ms: i64) -> String {
    let h = host(url);
    if h.ends_with("youtube.com") || h == "youtu.be" {
        format!("{url}{}t={}s", if url.contains('?') { '&' } else { '?' }, ms / 1000)
    } else {
        url.to_string()
    }
}

pub fn length(secs: f64) -> String {
    let m = (secs / 60.0).round() as u64;
    match m {
        0 => "under a minute".into(),
        1..=59 => format!("{m} min"),
        _ => format!("{} h {} min", m / 60, m % 60),
    }
}

pub fn ago(created: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    match now.saturating_sub(created) / 86_400 {
        0 => "today".into(),
        1 => "yesterday".into(),
        d => format!("{d} days ago"),
    }
}

pub fn open(url: &str) {
    #[cfg(windows)]
    let _ = hidden("explorer").arg(url).spawn();
    #[cfg(not(windows))]
    let _ = Command::new("open").arg(url).spawn();
}

pub fn file_name(title: &str, ext: &str) -> String {
    let clean: String = title.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { ' ' } else { c }).collect();
    format!("{}.{ext}", clean.trim().chars().take(80).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(clock(83_000), "1:23");
        assert_eq!(clock(3_723_000), "1:02:03");
        assert_eq!(srt_time(3_723_456), "01:02:03,456");
        assert_eq!(host("https://www.youtube.com/watch?v=x"), "youtube.com");
        assert_eq!(jump_url("https://www.youtube.com/watch?v=x", 61_500), "https://www.youtube.com/watch?v=x&t=61s");
        assert_eq!(jump_url("https://youtu.be/x", 5_000), "https://youtu.be/x?t=5s");
        assert_eq!(jump_url("https://vimeo.com/1", 5_000), "https://vimeo.com/1");
        assert_eq!(file_name("a/b: c?", "srt"), "a b  c.srt");
        assert!(explain("ERROR: [generic] Unsupported URL: x").starts_with("This page doesn't"));
    }

    /// Needs a fetched clip in the data dir and the fast model; run with --ignored --nocapture.
    #[test]
    #[ignore]
    fn real_audio() {
        let a = decode(&data().join("tmp/audio.m4a")).unwrap();
        println!("samples {} = {:.1}s", a.len(), a.len() as f32 / 16_000.0);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let segs = transcribe(&a, &dir("models").join(Model::Fast.file()), &tx, &Arc::new(AtomicBool::new(false)));
        println!("{segs:?}");
        assert!(!segs.unwrap().is_empty());
    }

    #[test]
    fn live_helpers() {
        assert!(fetchable("https://www.youtube.com/watch?v=x"));
        assert!(fetchable("https://player.vimeo.com/video/1"));
        assert!(!fetchable("https://canvas.myschool.edu/courses/1/lecture"));
        assert!(!fetchable("https://notyoutube.com/x"));
        let mut w = vec![0.5f32; 16_000 * 10];
        w[16_000 * 8..16_000 * 8 + 1_600].fill(0.0); // a pause at 8 s
        assert_eq!(quiet_cut(&w), 16_000 * 8 + 800);
    }

    /// Opens a process-loopback capture on this test process for a moment; needs Windows 10 2004+.
    #[test]
    #[ignore]
    fn capture_opens() {
        let finish = Arc::new(AtomicBool::new(false));
        let f = finish.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(800));
            f.store(true, Relaxed);
        });
        let buf = Arc::new(std::sync::Mutex::new(vec![]));
        capture(std::process::id(), buf, Arc::new(std::sync::atomic::AtomicU32::new(0)), finish).unwrap();
    }

    /// Plays a WAV from a hidden PowerShell and transcribes it live, the way a class portal tab would be.
    /// Run with TABSCRIBE_WAV=<file.wav> cargo test --release live_end_to_end -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_end_to_end() {
        let wav = std::env::var("TABSCRIBE_WAV").expect("set TABSCRIBE_WAV");
        let player = hidden("powershell")
            .args(["-NoProfile", "-Command", &format!("(New-Object Media.SoundPlayer '{wav}').PlaySync()")])
            .spawn()
            .unwrap();
        let finish = Arc::new(AtomicBool::new(false));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let t = Transcript::new("https://class.example.edu/lecture", None, Model::Fast);
        let id = t.id;
        let f = finish.clone();
        let worker = std::thread::spawn(move || listen(t, player.id(), tx, f));
        std::thread::sleep(Duration::from_secs(23));
        finish.store(true, Relaxed);
        worker.join().unwrap();
        let _ = fs::remove_file(dir("library").join(format!("{id}.json")));
        let mut done = None;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                Ev::Done(t) => done = Some(t),
                Ev::Failed(e) => panic!("{e}"),
                _ => {}
            }
        }
        let t = done.expect("finished");
        println!("{:#?}", t.segs);
        assert!(t.segs.iter().any(|s| s.text.to_lowercase().contains("elephant")));
    }

    #[test]
    fn resample_length() {
        let mut r = Mono16k::new(48_000).unwrap();
        r.push(std::iter::repeat(0.1).take(48_000 * 3));
        let n = r.finish().len() as i64;
        assert!((n - 48_000).abs() < 2_048, "3 s at 48 kHz should give ~48k samples at 16 kHz, got {n}");
    }
}
