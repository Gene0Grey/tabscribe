#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod chrome;
mod engine;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use engine::{Ev, Stage, Transcript};

const INDEX: &str = include_str!("../assets/index.html");
const FONTS: &str = include_str!("../assets/fonts.css");
const CSS: &str = include_str!("../assets/style.css");
const BOOT: &str = include_str!("../assets/boot.js");
// Keep the newest line in view while it is being written, unless the reader scrolled up.
const STICK: &str = "const l=document.getElementById('lines'); if(l && l.scrollHeight-l.scrollTop-l.clientHeight<160) l.scrollTop=l.scrollHeight;";

fn main() {
    let window = WindowBuilder::new()
        .with_title("tabscribe")
        .with_inner_size(LogicalSize::new(1040.0, 740.0))
        .with_min_inner_size(LogicalSize::new(380.0, 520.0));
    let cfg = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_background_color((0, 0, 0, 255))
        .with_disable_context_menu(!cfg!(debug_assertions))
        .with_custom_index(INDEX.to_string())
        .with_custom_head(format!("<style>{FONTS}{CSS}</style>"));
    dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

#[derive(Clone, PartialEq)]
enum Job {
    Idle,
    Running { stage: Stage, live: bool },
    Failed(String),
}

#[derive(Clone, Copy, PartialEq)]
enum View {
    Transcribe,
    History,
}

#[component]
fn App() -> Element {
    let demo = use_hook(preview::state);
    let mut tab = use_signal(|| demo.as_ref().and_then(|d| d.0.clone()));
    let mut job = use_signal(|| demo.as_ref().map_or(Job::Idle, |d| d.1.clone()));
    let mut current = use_signal(|| demo.as_ref().and_then(|d| d.2.clone()));
    let mut library = use_signal(|| demo.as_ref().map_or_else(engine::load_library, |d| d.3.clone()));
    let mut model = use_signal(engine::load_model);
    let mut view = use_signal(|| demo.as_ref().map_or(View::Transcribe, |d| d.4));
    let mut link = use_signal(|| None::<String>); // Some = the paste-a-link field is open
    let mut query = use_signal(String::new);
    let mut eta = use_signal(|| None::<(Instant, f32)>);
    let mut copied = use_signal(|| false);
    let cancel = use_hook(|| Arc::new(AtomicBool::new(false)));

    use_hook(|| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        chrome::watch(tx);
        spawn(async move {
            while let Some(t) = rx.recv().await {
                if matches!(*job.peek(), Job::Failed(_)) {
                    job.set(Job::Idle); // a new tab clears the old error
                }
                tab.set(t);
            }
        });
    });

    let tx = use_hook(|| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Ev>();
        spawn(async move {
            while let Some(ev) = rx.recv().await {
                match ev {
                    Ev::Stage(stage) => {
                        if let Stage::Listening(p) = stage {
                            if eta.peek().is_none() {
                                eta.set(Some((Instant::now(), p)));
                            }
                        }
                        let live = matches!(*job.peek(), Job::Running { live: true, .. });
                        job.set(Job::Running { stage, live });
                    }
                    Ev::Title(t, d) => {
                        if let Some(c) = current.write().as_mut() {
                            c.title = t;
                            c.duration = d;
                        }
                    }
                    Ev::Seg(s) => {
                        if let Some(c) = current.write().as_mut() {
                            c.segs.push(s);
                        }
                    }
                    Ev::Done(t) => {
                        library.write().insert(0, t.clone());
                        current.set(Some(t));
                        job.set(Job::Idle);
                    }
                    Ev::Failed(e) => job.set(Job::Failed(e)),
                    Ev::Stopped => job.set(Job::Idle),
                }
            }
        });
        tx
    });

    // `pid` = Some(browser) means: listen to the browser live instead of downloading.
    let start = {
        let cancel = cancel.clone();
        use_callback(move |(url, title, pid): (String, Option<String>, Option<u32>)| {
            let url = url.trim().to_string();
            if url.is_empty() || matches!(*job.peek(), Job::Running { .. }) {
                return;
            }
            let url = if url.contains("://") { url } else { format!("https://{url}") };
            let seed = Transcript::new(&url, title, *model.peek());
            current.set(Some(seed.clone()));
            query.set(String::new());
            link.set(None);
            eta.set(None);
            cancel.store(false, Ordering::Relaxed);
            let live = pid.is_some();
            let first = if live { Stage::Live { secs: 0.0, level: 0.0 } } else { Stage::Fetching(None) };
            job.set(Job::Running { stage: first, live });
            let (tx, cancel) = (tx.clone(), cancel.clone());
            std::thread::spawn(move || match pid {
                Some(pid) => engine::listen(seed, pid, tx, cancel),
                None => engine::run(seed, tx, cancel),
            });
        })
    };
    let start_tab = move |_| {
        if let Some(t) = tab() {
            let pid = (!engine::fetchable(&t.url)).then_some(t.pid);
            start((t.url, Some(t.title), pid));
        }
    };
    let start_link = move || {
        if let Some(l) = link() {
            start((l, None, None));
        }
    };

    use_effect(|| {
        document::eval(BOOT);
    });
    use_effect(move || {
        let _ = current.read().as_ref().map(|c| c.segs.len());
        if matches!(*job.peek(), Job::Running { .. }) {
            document::eval(STICK);
        }
    });

    let running = matches!(job(), Job::Running { .. });
    let m = model();
    // the tab on screen was already transcribed: say so instead of inviting a second 48-minute run
    let done = match (tab(), current()) {
        (Some(t), Some(c)) if t.url == c.url && !c.segs.is_empty() => Some(c.segs.len()),
        _ => None,
    };

    rsx! {
        div { class: "app",
            nav { class: "pivot", role: "tablist", "aria-label": "sections",
                for (v , label) in [(View::Transcribe, "transcribe"), (View::History, "history")] {
                    button {
                        role: "tab",
                        "aria-selected": "{view() == v}",
                        class: if view() == v { "pivot-item on" } else { "pivot-item" },
                        onclick: move |_| view.set(v),
                        "{label}"
                    }
                }
            }

            if view() == View::Transcribe {
                main { class: "page",
                    {source(&job(), &tab(), done, eta(), start_tab)}

                    div { class: "under",
                        match (job(), link()) {
                            (Job::Running { live, .. }, _) => rsx! {
                                p { class: "hint",
                                    if live { "keep the video playing with its sound on. press finish when the class ends." } else { "you can keep working; this runs on its own." }
                                }
                                button {
                                    class: "btn",
                                    onclick: {
                                        let cancel = cancel.clone();
                                        move |_| cancel.store(true, Ordering::Relaxed)
                                    },
                                    if live { "finish" } else { "stop" }
                                    {icon(ICON_STOP, true)}
                                }
                            },
                            (Job::Failed(_), _) => rsx! {
                                button { class: "btn", disabled: tab().is_none(), onclick: move |e| { job.set(Job::Idle); start_tab(e) }, "try again" }
                                button { class: "btn btn-quiet", onclick: move |_| job.set(Job::Idle), "dismiss" }
                            },
                            (Job::Idle, Some(l)) => rsx! {
                                label { class: "sr", r#for: "link", "video link" }
                                input {
                                    id: "link",
                                    class: "input",
                                    r#type: "url",
                                    placeholder: "paste a video link",
                                    spellcheck: "false",
                                    autofocus: true,
                                    value: "{l}",
                                    oninput: move |e| link.set(Some(e.value())),
                                    onkeydown: move |e| {
                                        if e.key() == Key::Enter {
                                            start_link()
                                        } else if e.key() == Key::Escape {
                                            link.set(None)
                                        }
                                    },
                                }
                                button { class: "btn", disabled: l.trim().is_empty(), onclick: move |_| start_link(), "go" {icon(ICON_PLAY, true)} }
                                button { class: "btn btn-quiet", onclick: move |_| link.set(None), "cancel" }
                            },
                            (Job::Idle, None) => rsx! {
                                if done.is_some() {
                                    p { class: "hint", "already transcribed. it's below, and saved in history." }
                                    button { class: "btn btn-quiet", onclick: start_tab, "transcribe again" }
                                } else {
                                    p { class: "hint", {hint(&tab())} }
                                }
                                button { class: "btn btn-quiet", onclick: move |_| link.set(Some(String::new())), "paste a link" }
                            },
                        }
                    }

                    if let Some(c) = current() {
                        {document_view(c, running, &mut query, &mut copied)}
                    }
                }
            } else {
                main { class: "page",
                    if library.read().is_empty() {
                        p { class: "hint big", "your transcripts are kept here, on this pc. none yet." }
                    } else {
                        ul { class: "history vscroll",
                            for t in library.read().iter() {
                                li { key: "{t.id}",
                                    button {
                                        class: "row",
                                        disabled: running,
                                        onclick: {
                                            let t = t.clone();
                                            move |_| {
                                                current.set(Some(t.clone()));
                                                query.set(String::new());
                                                view.set(View::Transcribe);
                                            }
                                        },
                                        span { class: "row-title", "{t.title}" }
                                        span { class: "row-meta", "{engine::host(&t.url)} · {engine::length(t.duration)} · {engine::ago(t.created)}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            footer { class: "foot",
                span { "runs entirely on this pc. nothing is uploaded." }
                button {
                    class: "foot-model",
                    disabled: running,
                    title: "{m.detail()}. press to switch between fast, balanced and accurate",
                    onclick: move |_| {
                        let n = model().next();
                        model.set(n);
                        engine::save_model(n);
                    },
                    "speech model: "
                    strong { "{m.label()}" }
                    if !engine::model_ready(m) { span { class: "dim", " · downloads once, {m.size()}" } }
                    span { class: "change", "change" }
                }
            }
        }
    }
}

/// The one big tile: the tab waiting to be transcribed, the job in progress, or what went wrong.
/// Metro order: the big words first, the small facts under them.
fn source(job: &Job, tab: &Option<chrome::Tab>, done: Option<usize>, eta: Option<(Instant, f32)>, start: impl FnMut(MouseEvent) + Copy + 'static) -> Element {
    match job {
        Job::Running { stage, .. } => {
            let (word, detail, prog) = stage_text(stage, eta);
            rsx! {
                div { class: "src src-busy",
                    span { class: "src-stage", role: "status", "{word}" }
                    span { class: "src-meta", "{detail}" }
                    match prog {
                        Prog::Bar(p) => rsx! { span { class: "bar", i { style: format!("transform:scaleX({:.3})", p / 100.0) } } },
                        Prog::Busy => rsx! { span { class: "dots", i {} i {} i {} i {} i {} } },
                        Prog::Still => rsx! { span { class: "bar bar-still" } },
                    }
                }
            }
        }
        Job::Failed(msg) => rsx! {
            div { class: "src src-fail", role: "alert",
                span { class: "src-stage", "couldn't transcribe" }
                span { class: "src-meta", "{msg}" }
            }
        },
        Job::Idle => match (tab, done) {
            (Some(t), Some(n)) => rsx! {
                div { class: "src src-done",
                    span { class: "src-stage", "transcribed" }
                    span { class: "src-meta", "{t.title} · {n} lines" }
                }
            },
            // keyed so a different tab remounts and flips in, like a live tile
            (Some(t), None) => rsx! {
                for t in std::iter::once(t) {
                    button { key: "{t.url}", class: "src src-tab src-action tilt", onclick: start,
                        span { class: "src-title", "{t.title}" }
                        span { class: "src-foot",
                            span { class: "src-meta", "{engine::host(&t.url)}" }
                            span { class: "src-go", "transcribe" {icon(ICON_PLAY, true)} }
                        }
                    }
                }
            },
            (None, _) => rsx! {
                div { class: "src src-empty",
                    span { class: "src-title", "open a class or video" }
                    span { class: "src-meta", "in chrome, edge or brave. it shows up here on its own." }
                }
            },
        },
    }
}
fn hint(tab: &Option<chrome::Tab>) -> &'static str {
    match tab {
        Some(t) if engine::fetchable(&t.url) => "this site is downloaded directly, so you don't need to play the video.",
        Some(_) => "after you press it, play the video in your browser. the text is written as it plays.",
        None => "or paste a link to a public video.",
    }
}

fn document_view(c: Transcript, running: bool, query: &mut Signal<String>, copied: &mut Signal<bool>) -> Element {
    let (mut query, mut copied) = (*query, *copied);
    let has = !c.segs.is_empty();
    let q = query();
    let needle = q.to_lowercase();
    let hits = c.segs.iter().enumerate().filter(|(_, s)| needle.is_empty() || s.text.to_lowercase().contains(&needle)).collect::<Vec<_>>();
    rsx! {
        section { class: "doc", "aria-label": "transcript",
            div { class: "doc-head",
                div { class: "doc-titles",
                    h1 { class: "doc-title", title: "{c.title}", "{c.title}" }
                    p { class: "doc-meta", {meta(&c)} }
                }
                div { class: "tools", role: "toolbar", "aria-label": "transcript actions",
                    button {
                        class: "tool",
                        "aria-label": "copy transcript",
                        disabled: !has,
                        onclick: {
                            let text = engine::to_plain(&c);
                            move |_| {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    copied.set(cb.set_text(text.clone()).is_ok());
                                }
                            }
                        },
                        onmouseleave: move |_| copied.set(false),
                        {icon(ICON_COPY, false)}
                        span { class: "tool-label", if copied() { "copied" } else { "copy" } }
                    }
                    {save_button("save txt", ICON_TXT, has && !running, engine::file_name(&c.title, "txt"), engine::to_txt(&c))}
                    {save_button("save subtitles", ICON_SRT, has && !running, engine::file_name(&c.title, "srt"), engine::to_srt(&c))}
                    button {
                        class: "tool",
                        onclick: {
                            let url = c.url.clone();
                            move |_| engine::open(&url)
                        },
                        {icon(ICON_OPEN, false)}
                        span { class: "tool-label", "open" }
                    }
                    if has {
                        div { class: "search",
                            {icon(ICON_SEARCH, false)}
                            input {
                                class: "input",
                                r#type: "search",
                                placeholder: "find",
                                "aria-label": "find in transcript",
                                value: "{q}",
                                oninput: move |e| query.set(e.value()),
                            }
                        }
                    }
                }
            }
            div { class: if running { "lines vscroll live" } else { "lines vscroll" }, id: "lines",
                for (i , s) in hits.iter().copied() {
                    div { key: "{i}", class: "line",
                        button {
                            class: "ts",
                            title: "open the video at {engine::clock(s.t0)}",
                            onclick: {
                                let u = engine::jump_url(&c.url, s.t0);
                                move |_| engine::open(&u)
                            },
                            "{engine::clock(s.t0)}"
                        }
                        p { {highlight(&s.text, &needle)} }
                    }
                }
                if !has && running {
                    p { class: "hint", "lines appear here as they are heard." }
                }
                if has && hits.is_empty() {
                    p { class: "hint", "no line says “{q}”." }
                }
            }
        }
    }
}

enum Prog {
    Bar(f32),
    Busy,
    Still, // honest waiting: nothing is happening, so nothing moves
}

fn stage_text(stage: &Stage, eta: Option<(Instant, f32)>) -> (&'static str, String, Prog) {
    match stage {
        Stage::Setup { what, pct, mb } => {
            let size = match (pct, mb) {
                (Some(p), mb) if *mb > 0 => format!(" · {p:.0}% of {mb} MB"),
                (Some(p), _) => format!(" · {p:.0}%"),
                _ => String::new(),
            };
            ("getting ready", format!("downloading the {what}, one time only{size}"), pct.map_or(Prog::Busy, Prog::Bar))
        }
        Stage::Updating => ("getting ready", "checking the video fetcher for updates".into(), Prog::Busy),
        Stage::Fetching(None) => ("fetching", "finding the audio".into(), Prog::Busy),
        Stage::Fetching(Some(p)) => ("fetching", format!("downloading the audio · {p:.0}%"), Prog::Bar(*p)),
        Stage::Reading => ("reading", "preparing the sound".into(), Prog::Busy),
        Stage::Listening(p) => {
            let left = eta
                .filter(|(_, p0)| p - p0 >= 3.0)
                .map(|(t0, p0)| {
                    let secs = t0.elapsed().as_secs_f32() * (100.0 - p) / (p - p0);
                    if secs < 60.0 { " · under a minute left".to_string() } else { format!(" · about {} min left", (secs / 60.0).round()) }
                })
                .unwrap_or_default();
            ("transcribing", format!("{p:.0}% done{left}"), Prog::Bar(*p))
        }
        Stage::Live { secs, level } => {
            let at = engine::clock((*secs * 1000.0) as i64);
            if *level > 0.002 {
                ("listening", format!("{at} recorded · hearing the browser"), Prog::Busy)
            } else {
                ("listening", format!("{at} recorded · no sound yet: press play in your browser"), Prog::Still)
            }
        }
        Stage::Finishing => ("finishing", "writing down the last part".into(), Prog::Busy),
    }
}
fn meta(c: &Transcript) -> String {
    let mut parts = vec![engine::host(&c.url)];
    if c.duration > 0.0 {
        parts.push(engine::length(c.duration));
    }
    if !c.segs.is_empty() {
        parts.push(format!("{} lines", c.segs.len()));
    }
    parts.join(" · ")
}

fn highlight(text: &str, needle: &str) -> Element {
    let low = text.to_lowercase();
    // byte offsets only line up when lowercasing kept the length; otherwise skip the marks
    if needle.is_empty() || low.len() != text.len() {
        return rsx! { "{text}" };
    }
    let mut parts = vec![];
    let mut at = 0;
    while let Some(i) = low[at..].find(needle) {
        let s = at + i;
        parts.push((false, &text[at..s]));
        parts.push((true, &text[s..s + needle.len()]));
        at = s + needle.len();
    }
    parts.push((false, &text[at..]));
    rsx! {
        for (hit , p) in parts {
            if hit { mark { "{p}" } } else { "{p}" }
        }
    }
}

fn save_button(label: &'static str, path: &'static str, enabled: bool, name: String, body: String) -> Element {
    rsx! {
        button {
            class: "tool",
            disabled: !enabled,
            onclick: move |_| {
                let (name, body) = (name.clone(), body.clone());
                spawn(async move {
                    let ext = name.rsplit('.').next().unwrap_or("txt").to_string();
                    if let Some(f) = rfd::AsyncFileDialog::new().set_file_name(&name).add_filter(&ext, &[&ext]).save_file().await {
                        let _ = std::fs::write(f.path(), body);
                    }
                });
            },
            {icon(path, false)}
            span { class: "tool-label", "{label}" }
        }
    }
}

fn icon(d: &'static str, solid: bool) -> Element {
    rsx! {
        svg { class: if solid { "ic ic-solid" } else { "ic" }, view_box: "0 0 24 24", "aria-hidden": "true",
            path { d }
        }
    }
}

const ICON_PLAY: &str = "M7 4.5 19 12 7 19.5z";
const ICON_STOP: &str = "M6 6h12v12H6z";
const ICON_COPY: &str = "M9 9h11v11H9zM15 9V4H4v11h5";
const ICON_TXT: &str = "M6 3h8l4 4v14H6zM14 3v4h4M9 12h6M9 15.5h6";
const ICON_SRT: &str = "M3 6h18v12H3zM6.5 13.5h5M13.5 13.5h4M6.5 10.5h3M11.5 10.5h6";
const ICON_OPEN: &str = "M14 4h6v6M20 4l-9 9M18 14v6H4V6h6";
const ICON_SEARCH: &str = "M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13zM15.5 15.5 20 20";

/// Renders the main states to HTML so the design can be reviewed without driving the real window.
/// `cargo test --release preview` writes target/preview/*.html. Every transcript in it is synthetic.
mod preview {
    use super::*;

    type State = Option<(Option<chrome::Tab>, Job, Option<Transcript>, Vec<Transcript>, View)>;

    #[cfg(not(test))]
    pub fn state() -> State {
        None
    }

    #[cfg(test)]
    pub fn state() -> State {
        let name = std::env::var("TABSCRIBE_PREVIEW").ok()?;
        let tab = chrome::Tab {
            title: "Lecture 12: Entropy and the second law".into(),
            url: "https://canvas.myschool.edu/courses/2231/modules/items/88".into(),
            pid: 0,
        };
        let mut t = Transcript::new(&tab.url, Some(tab.title.clone()), engine::Model::Balanced);
        t.duration = 2_860.0;
        let lines = [
            "Okay, let's get started. Last week we left off with the first law, energy is conserved.",
            "Today the question is different: not how much energy, but which way things go.",
            "Think about an ice cube on a warm table. Nobody has ever seen it get colder on its own.",
            "That one-way street is what entropy measures.",
            "Boltzmann's version is the one I want you to remember: S equals k log W.",
            "W is the number of microscopic arrangements that look the same from the outside.",
            "So a shuffled deck has enormous W, and a sorted deck has exactly one arrangement.",
            "When we say entropy increases, we mean the system drifts toward the larger W.",
            "Not because of a force pushing it, but because there are simply more ways to be there.",
            "Write this down, because it shows up on the midterm in some form every year.",
        ];
        t.segs = lines.iter().enumerate().map(|(i, s)| engine::Seg { t0: i as i64 * 7_400, t1: (i as i64 + 1) * 7_400, text: s.to_string() }).collect();
        let mut older = t.clone();
        older.title = "Lecture 11: The first law and heat engines".into();
        older.created -= 7 * 86_400;
        older.url = "https://canvas.myschool.edu/courses/2231/modules/items/71".into();
        let mut yt = t.clone();
        yt.title = "Carnot cycle explained in 10 minutes".into();
        yt.url = "https://www.youtube.com/watch?v=demo".into();
        yt.duration = 610.0;
        yt.created -= 86_400;
        let lib = vec![t.clone(), yt, older];
        Some(match name.as_str() {
            "empty" => (None, Job::Idle, None, vec![], View::Transcribe),
            "idle" => (Some(tab), Job::Idle, Some(lib[2].clone()), lib, View::Transcribe),
            "done" => (Some(tab), Job::Idle, Some(lib[0].clone()), lib, View::Transcribe),
            "live" => {
                let mut part = t.clone();
                part.segs.truncate(6);
                (Some(tab), Job::Running { stage: Stage::Live { secs: 754.0, level: 0.35 }, live: true }, Some(part), lib, View::Transcribe)
            }
            "live-quiet" => {
                let mut part = t.clone();
                part.segs.truncate(6);
                (Some(tab), Job::Running { stage: Stage::Live { secs: 754.0, level: 0.0 }, live: true }, Some(part), lib, View::Transcribe)
            }
            "fail" => (Some(tab), Job::Failed("Nothing was heard. Start the video in your browser with its sound on, then try again.".into()), None, lib, View::Transcribe),
            "history" => (Some(tab), Job::Idle, None, lib, View::History),
            _ => return None,
        })
    }

    #[cfg(test)]
    #[test]
    fn render() {
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/preview");
        std::fs::create_dir_all(&out).unwrap();
        for name in ["empty", "idle", "done", "live", "live-quiet", "fail", "history"] {
            unsafe { std::env::set_var("TABSCRIBE_PREVIEW", name) };
            let mut dom = VirtualDom::new(App);
            dom.rebuild_in_place();
            let body = dioxus_ssr::render(&dom);
            let page = INDEX.replace("<!-- CUSTOM HEAD -->", &format!("<style>{FONTS}{CSS}</style>")).replace("<div id=\"main\"></div>", &format!("<div id=\"main\">{body}</div>"));
            std::fs::write(out.join(format!("{name}.html")), page).unwrap();
        }
    }
}