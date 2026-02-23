# doing-now

A lightweight macOS floating app that keeps your current task visible at all times.

## Overview

Trigger a global shortcut to display what you're working on right now. The bubble is click-through by default, so it never gets in your way.

```text
╔══════════════════════╗
║  Pinned to top-left  ║
║  ┌──────────────┐    ║
║  │ Design review │   ║
║  └──────────────┘    ║
║   (click-through)    ║
╚══════════════════════╝
```

## Download

Visit **[senri1101.github.io/notch-now](https://senri1101.github.io/notch-now/)** and click the download button. Drag **notch-now.app** into your Applications folder.

> **Note:** The app is not code-signed or notarized. macOS will show a "damaged" error on first launch.
> Remove the quarantine attribute with Terminal, then open normally:
>
> ```bash
> xattr -cr ~/Downloads/notch-now.app
> ```

## Usage

| Action | Effect |
| ------ | ------ |
| `⌥ ⌘ Space` (once) | Briefly highlight the bubble |
| `⌥ ⌘ Space` (double-tap) | Enter edit mode |
| `Enter` | Save and close |
| `Escape` | Cancel edit |

- Text is capped at **20 characters**
- Displays **"Now"** when empty
- Text is persisted to `localStorage` automatically

## Development

### Prerequisites

- [Node.js](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (macOS: Xcode Command Line Tools)

### Setup

```bash
npm install
```

### Run (dev mode)

```bash
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

## TODO

- [ ] **E1: Code Signing & Notarization**
  The app is currently unsigned; macOS Gatekeeper will show a warning on first launch.
  After joining the Apple Developer Program, configure `bundle.macOS.signingIdentity` and `bundle.macOS.notarize` in `tauri.conf.json`.
  See: [Tauri - Sign macOS apps](https://v2.tauri.app/distribute/sign/macos/)

## Tech Stack

- [Tauri 2](https://v2.tauri.app/) — Rust-based desktop app framework
- [TypeScript](https://www.typescriptlang.org/) + [Vite](https://vite.dev/) — frontend
- Rust — backend (global shortcut, window management)
