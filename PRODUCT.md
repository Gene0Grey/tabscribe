# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Windows desktop app; UI renders in the system WebView2 via Dioxus desktop, so the design language is web. macOS later.)

## Stack

Rust throughout. Dioxus desktop for the UI (RSX components + CSS). whisper.cpp via `whisper-rs` for local speech-to-text. WASAPI process-loopback capture (`wasapi` crate) to hear the browser. `yt-dlp` sidecar binary for downloading public videos. Windows UI Automation to read the active browser tab. Windows first; code kept portable for macOS.

## Users

General public, mainly students: non-technical people who install the app from an installer and never open a terminal. The main scene is a recorded class in a logged-in course portal (Canvas-style LMS, lecture capture players) that they want as text without re-watching it. Public videos (YouTube and similar) are the secondary case.

## Product Purpose

Turn the class or video in the current browser tab into a full, timestamped transcript, entirely on the user's machine. Secondary path: paste a public video link. Success = one action from "I'm watching this" to "I have the whole transcript", with no account, no upload, no setup.

## Positioning

Fully local: audio never leaves the machine, no account, no API key, no usage cap. It reads the tab you are already on and works behind your school login, because it listens to what the browser plays instead of trying to download from the portal.

## Operating Context

- User has the class open in Chrome, Edge or Brave, switches to the app (or snaps it beside the browser) and presses the tab tile.
- Class portals and any other non-public site: the app listens to the browser's sound live while the user plays the video, and writes the transcript as it plays (real time; playing at higher speed shortens it). The user presses finish when the class ends.
- Public video sites (YouTube, Vimeo, and similar): the audio is downloaded directly and transcribed faster than real time; no need to play it.
- First run downloads the speech model (and, for public sites, yt-dlp) once; this is explained, with size and progress.
- Long classes (1-3 h) are common, so progress, time estimates and "already transcribed" state matter.
- Output is copied, saved as .txt / subtitles (.srt), or searched inside the app; every transcript is kept in history on the PC.

## Capabilities and Constraints

- Tab detection via Windows UI Automation for Chrome, Edge and Brave; no extension, no browser flags.
- Live listening captures the whole browser process (all its tabs), so other tabs playing sound are captured too. The video must play with its sound on.
- Logins are handled by listening, not by an error. DRM-protected video is also heard this way.
- CPU-only assumed (no GPU on the dev machine). Model size choice trades speed vs accuracy.
- Languages: whisper multilingual models; auto-detect language.
- Undecided: auto-update mechanism, installer format, history persistence location, CPU baseline for distributed builds.

## Evidence on Hand

None yet: no users, testimonials, benchmarks or speed claims. Do not invent accuracy or speed figures.

## Product Principles

1. One action. The default path needs zero typing: detect the tab, press, done.
2. Private by construction. Never imply anything is uploaded; say plainly that it runs locally.
3. Honest waiting. Long jobs show real stage, progress and what is happening, never a fake spinner.
4. Plain language. Errors tell a non-technical person what happened and what to do next.

## Accessibility & Inclusion

Keyboard operable throughout; readable transcript text at comfortable size; respects Windows reduced-motion setting. Target WCAG 2.2 AA contrast.
