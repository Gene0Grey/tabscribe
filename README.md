# tabscribe

Transcribe the video in your current browser tab, entirely on your own PC. Written in Rust.

- **Class portals and any site behind a login:** press the tile, play the video, and tabscribe listens to the browser's sound and writes the transcript as it plays.
- **YouTube, Vimeo and other public sites:** the audio is downloaded directly and transcribed faster than real time.
- Nothing is uploaded. Speech recognition is [whisper.cpp](https://github.com/ggml-org/whisper.cpp) running locally.
- Copy, search, save as .txt or subtitles (.srt); every transcript is kept in history.

Windows 10 (2004) or later, with Chrome, Edge or Brave. The speech model (147 to 574 MB) downloads once on first use.

## Build

Needs Rust (MSVC toolchain), Visual Studio C++ build tools, CMake, and libclang.

```bash
pip install libclang
```

```powershell
$env:LIBCLANG_PATH = "<python>\Lib\site-packages\clang\native"
cargo build --release
```

The app is `target\release\tabscribe.exe`.

```bash
cargo test --release
```

`cargo test --release preview` renders every screen state to `target/preview/*.html`.
