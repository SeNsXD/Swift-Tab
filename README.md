<div align="center">

<img src="assets/readme/hero.png" width="880" alt="Swift Tab in the Windows taskbar with music, Claude Code, timer, weather, downloads, and sports pills">

# Swift Tab

**A fluid, dynamic status pill and expandable companion cards embedded right into the Windows taskbar.**

[![Release](https://img.shields.io/github/v/release/SeNsXD/Swift-Tab?style=flat-square&color=0078D4)](https://github.com/SeNsXD/Swift-Tab/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%2011%20%7C%2010-0078D4?style=flat-square&logo=windows)](https://github.com/SeNsXD/Swift-Tab)
[![Language](https://img.shields.io/badge/built%20with-Rust-dea584?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue?style=flat-square)](LICENSE)

Swift Tab sits quietly in your taskbar, showing what matters most right now — active downloads, playing tracks with live lyrics, countdown timers, weather conditions, sports scores, and AI coding steps. Click or hover the pill to smoothly expand rich interactive cards.

[**Download Latest Release**](https://github.com/SeNsXD/Swift-Tab/releases/latest) &nbsp;·&nbsp;
[Features](#features) &nbsp;·&nbsp;
[Keyboard Shortcuts](#keyboard-shortcuts) &nbsp;·&nbsp;
[Installation](#installation) &nbsp;·&nbsp;
[Building](#building-from-source) &nbsp;·&nbsp;
[Credits](#credits--acknowledgments)

</div>

<br>

---

## Highlights

<div align="center">
<img src="assets/readme/pills.png" width="880" alt="Examples of Swift Tab pills: music, Claude Code permission, match score, timer, weather, rain alert, downloads, shelf, and volume">
</div>

* **Native Windows Integration**: Built directly into the Windows Shell (`Shell_TrayWnd`) using modern Direct2D, DirectWrite, and Windows Composition APIs. Zero electron bloat, blazing fast rendering, and minimal resource usage (< 40 MB RAM idle).
* **Smart Priority Stacking**: Dynamically raises the most pertinent pill (e.g. an incoming rain alert, an active download, or an urgent permission request) while letting you scroll through active activities with your mouse wheel.
* **Auto-Hide & Fullscreen Friendly**: Automatically hides when gaming, watching full-screen videos, or giving presentations so it never gets in your way.

---

## Features

### ⬇️ Live Download Monitor
* **Real-time Tracking**: Automatically detects active browser downloads across Google Chrome, Microsoft Edge, Brave, Opera, and Firefox.
* **Live Speed & ETA**: Computes actual transfer rate, remaining time, and true byte progress (`1.4 MB / 4.7 GB (0%) · 722 KB/s · 49m left`).
* **Interactive Actions**:
  * **Open**: Immediately focuses and brings the downloading browser window to the front.
  * **Folder**: Opens the download folder in File Explorer and highlights the completed file.

### 🎵 Music & Synchronized Lyrics
* **Universal Media Sessions**: Works with Spotify, YouTube Music, Apple Music, Tidal, web browsers, and any media player supporting Windows Media Sessions.
* **Dynamic Album Accents**: The pill and card automatically sample album art palettes for fluent gradient backgrounds.
* **Live Scrolling Lyrics**: Synchronized time-stamped lyrics sourced in real-time from `lrclib.net`.
* **Output Selector**: Route audio between speakers, headphones, and Bluetooth outputs on the fly.
* **Weekly Music Digest**: Track your most listened-to tracks and artists over the past 7 days.

### 🌤️ Weather & Severe Alerts
* **Hourly Forecast**: Real-time temperature, condition summaries, and hour-by-hour forecast with clean vector weather icons.
* **Proactive Hazard Alerts**: High-priority alerts for rain, snow, thunderstorms, high winds, extreme heat, freeze warnings, and poor air quality.
* **Location Flexibility**: Auto-locates your city via Windows Location or allows setting a custom static location.

### 📋 Smart Clipboard
* **History Stash**: Keeps your recent text and image clips accessible in one place.
* **Quick Pinning**: Pin frequently needed snippets for quick re-use.
* **Plain Text Paste**: Paste unformatted text with one click.

### 📁 Drop Shelf
* **Drag-and-Drop Stash**: Drag files directly onto the pill to hold them temporarily before dropping them elsewhere.
* **Screenshot Auto-Capture**: New screenshots automatically land on the shelf.
* **One-Click Utilities**: Share to phone (Quick Share/Windows Share), OCR text extraction from images, downscale photos to 1080p, or batch Zip files.

### 🤖 Claude Code Companion
* **Terminal & CLI Companion**: Deep integration with Anthropic's Claude Code CLI.
* **Live Step Feedback**: Displays current actions, working files, and elapsed time (`Swift Tab · Editing: app.rs`).
* **Taskbar Permission Cards**: When Claude Code requests command or file execution permission, approve or deny directly from the taskbar card.
* **Ask Claude**: Press <kbd>Win</kbd>+<kbd>Alt</kbd>+<kbd>C</kbd> or <kbd>Ctrl</kbd>+Click the pill to open a prompt card and stream answers.

### 🎧 Galaxy Buds & Audio Devices
* **Galaxy Buds Support**: Live battery percentages for left earbud, right earbud, and cradle for Buds2, Buds2 Pro, Buds FE, Buds3, and Buds Pro.
* **Noise Control**: Toggle between Active Noise Canceling (ANC), Ambient Sound, and Off directly from the card.
* **Bluetooth Headsets**: Battery gauges for connected Bluetooth headphones.

### ⚽ Live Sports Tracker
* **Real-time Match Scores**: Live game clocks, current scores, and goal notifications for your favorite football and basketball teams (ESPN & EuroLeague).

### ⏱️ Timers & Stopwatch
* Quick countdown timers and precision stopwatch with alarm sound notifications and auto-expanding completion cards.

---

## Cards Showcase

<div align="center">
<img src="assets/readme/music.png" width="880" alt="Music card with lyrics and controls">
<br><br>
<img src="assets/readme/day.png" width="880" alt="Daily summary, timer, hourly weather, and wellbeing cards">
<br><br>
<img src="assets/readme/live.png" width="880" alt="Live match card, Claude Code permission card, and Galaxy Buds card">
</div>

---

## Keyboard Shortcuts

| Shortcut / Action | Function |
|---|---|
| **Click Pill** | Open or toggle the current card (smoothly closes when moving away) |
| **Middle Click Pill** | Play / Pause media playback |
| **Mouse Wheel on Pill** | Cycle through multiple active activities in the stack |
| **Right Click Pill** | Context menu (Open download folder, focus media app, etc.) |
| <kbd>Win</kbd> + <kbd>Alt</kbd> + <kbd>N</kbd> | Open quick timer and stopwatch picker |
| <kbd>Win</kbd> + <kbd>Alt</kbd> + <kbd>C</kbd> | Open **Ask Claude** input box |
| <kbd>Ctrl</kbd> + <kbd>Click Pill</kbd> | Quick shortcut to Ask Claude |
| <kbd>Ctrl</kbd> + <kbd>F</kbd> (in Settings) | Search settings, teams, or app exceptions |
| <kbd>Ctrl</kbd> + <kbd>K</kbd> (in Settings) | Open Command Palette |

---

## Installation

### Pre-built Binaries (Recommended)

1. Download the latest `swifttab-setup.exe` or standalone portable zip from [**Releases**](https://github.com/SeNsXD/Swift-Tab/releases/latest).
2. Run the installer. It installs locally to `%LOCALAPPDATA%\Programs\SwiftTab` without requiring administrator privileges.
3. Configure optional preferences:
   * **Start with Windows**
   * **Connect Claude Code hooks**
   * **Enable Desktop Shortcut**

*To install silently (e.g. via scripts or winget):*
```powershell
swifttab-setup.exe --quiet
```

---

## Building from Source

### Prerequisites
* **Rust**: Latest stable toolchain (`x86_64-pc-windows-msvc`).
* **Windows SDK**: For resource compilation (`rc.exe`).
* Windows 10 (version 1809+) or Windows 11.

### Quick Build
Clone the repository and build the quick development binaries:

```powershell
git clone https://github.com/SeNsXD/Swift-Tab.git
cd Swift-Tab
cargo build --profile quick --bin swifttab --bin swifttab-settings
```

Compiled binaries will be generated at:
* `target\quick\swifttab.exe` (main taskbar companion)
* `target\quick\swifttab-settings.exe` (settings GUI)

### Release Packaging
To build an optimized production installer and standalone distribution files:

```powershell
powershell -ExecutionPolicy Bypass -File tools\make-setup.ps1
```

---

## Privacy & Network Disclosure

Swift Tab does **not** track telemetry, record keystrokes, or collect personal data. Network interactions only occur for the features you explicitly choose to use:

* **Lyrics**: Sourced anonymously on-demand from [`lrclib.net`](https://lrclib.net) (can be disabled in Media settings).
* **Weather**: Queries [`open-meteo.com`](https://open-meteo.com) for weather conditions using city coordinates or Windows location.
* **Sports**: Queries public endpoints ([`site.api.espn.com`](https://site.api.espn.com) & EuroLeague) for live scores.
* **Ask Claude**: Handled entirely through your locally installed Claude Code CLI session (`claude -p`).
* **Update Checks**: Checks GitHub Releases API once per day.

---

## Credits & Acknowledgments

* **Concept & App Inspiration**: Special credit to the original **Now Bar** project for the brilliant initial companion concept.
* **Typography**: Powered by Segoe UI Variable and Inter (SIL Open Font License).
* **Settings UI**: Built using the [Slint](https://slint.dev/) declarative GUI framework.

---

## License

Swift Tab is open-source software licensed under the [GNU General Public License v3.0](LICENSE).
You are free to use, modify, and distribute this software according to the terms of the GPL-3.0 license.
