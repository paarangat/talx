<p align="center">
  <img src="src-tauri/icons/source-logo.png" alt="Talx Logo" width="120" />
</p>

<h1 align="center">Talx</h1>

<p align="center">
  <strong>Open-source, cross-platform voice-to-text dictation that works everywhere on your desktop.</strong>
</p>

<p align="center">
  <a href="#features">Features</a> &middot;
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="#configuration">Configuration</a> &middot;
  <a href="#contributing">Contributing</a> &middot;
  <a href="#roadmap">Roadmap</a>
</p>

<p align="center">
  <a href="https://github.com/paarangat/talx/blob/main/LICENSE"><img src="https://img.shields.io/github/license/paarangat/talx?color=b07a3b&style=flat-square" alt="License" /></a>
  <a href="https://github.com/paarangat/talx/stargazers"><img src="https://img.shields.io/github/stars/paarangat/talx?color=b07a3b&style=flat-square" alt="Stars" /></a>
  <a href="https://github.com/paarangat/talx/issues"><img src="https://img.shields.io/github/issues/paarangat/talx?color=c0503a&style=flat-square" alt="Issues" /></a>
  <a href="https://github.com/paarangat/talx/pulls"><img src="https://img.shields.io/github/issues-pr/paarangat/talx?color=5d8a4c&style=flat-square" alt="PRs" /></a>
</p>

---

Speak naturally in any app — Talx transcribes your speech in real-time, polishes it with AI, and pastes the result right where your cursor is. Supports English, Hindi, and mixed-language (code-switching) dictation.

Think [Wispr Flow](https://wisprflow.ai), but **open-source**, **self-hosted** with your own API keys, and **privacy-first**.

## Features

- **System-wide dictation** — works in any text field, any app
- **Real-time streaming** — see your words appear as you speak
- **AI polishing** — removes filler words, fixes grammar, matches context
- **Multilingual** — English, Hindi, and code-switching (Hinglish) out of the box
- **Privacy-first** — your API keys, your data, no middleman
- **Free tier** — Groq-powered ASR and LLM at zero cost
- **Cloud tier** — bring your own Soniox + OpenAI keys for streaming and higher accuracy
- **Lightweight** — ~8MB bundle, ~50MB RAM (vs 150MB+ for Electron apps)

## How it works

```
[You speak] → [ASR: Groq or Soniox] → [Raw transcript] → [LLM: Groq or OpenAI] → [Polished text] → [Pasted into active app]
     ↑                                                                                                          ↓
  Hold ⌥Space                                                                                           Release ⌥Space
```

1. **Hold** the hotkey (default: `⌥ Space`)
2. **Speak** naturally — the floating widget shows your words streaming in
3. **Release** the hotkey — AI cleans up the transcript
4. **Done** — polished text is auto-pasted into whatever app you're using

## Tech stack

| Layer | Technology | Why |
|-------|-----------|-----|
| Desktop framework | [Tauri v2](https://v2.tauri.app/) | Tiny bundle, native performance, cross-platform |
| Frontend | React + TypeScript + Vite | Fast dev, large contributor pool |
| Transcription (free) | [Groq Whisper](https://console.groq.com/) | Free, fast inference |
| Transcription (cloud) | [Soniox API](https://soniox.com/) | Best accuracy, code-switching, streaming |
| AI polishing (free) | [Groq LLM](https://console.groq.com/) | Free, fast inference |
| AI polishing (cloud) | [OpenAI API](https://platform.openai.com/) | High-quality text cleanup |
| Backend | Rust | System-level access (hotkeys, audio, paste) |

## Cost comparison

At typical usage (~30 min/day of dictation):

| | Wispr Flow Pro | Talx (self-hosted) |
|---|---|---|
| Monthly cost | $12/month | **Free** (Groq) / ~$2-3 (cloud keys) |
| Annual cost | $144/year | **Free** (Groq) / ~$24-36 (cloud keys) |
| Data privacy | Cloud (their servers) | Your API keys, always |
| Customization | None | Full control, open source |

## Prerequisites

- [Rust](https://rustup.rs/) (for the Tauri backend)
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- API keys (at least one tier):
  - **Free**: [Groq API key](https://console.groq.com/) — ASR + LLM at no cost
  - **Cloud**: [Soniox](https://console.soniox.com/) + [OpenAI](https://platform.openai.com/) — streaming + higher accuracy

<details>
<summary><strong>macOS</strong></summary>

```bash
xcode-select --install
```
</details>

<details>
<summary><strong>Linux (Ubuntu/Debian)</strong></summary>

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```
</details>

<details>
<summary><strong>Windows</strong></summary>

- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- WebView2 (pre-installed on Windows 10/11)
</details>

## Quick start

```bash
# Clone the repo
git clone https://github.com/paarangat/talx.git
cd talx

# Install dependencies
pnpm install

# Set up your API keys (first time only)
cp .env.example .env
# Edit .env with your API keys

# Run in development mode
pnpm tauri dev
```

On first launch, Talx will prompt you to enter your API keys. They're stored securely in your OS keychain.

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| Hotkey | `⌥ Space` | Hold to record, release to process |
| Language | `en, hi` | Languages for transcription |
| API tier | `free` | `free` (Groq) or `cloud` (Soniox + OpenAI) |
| LLM model | `llama-3.3-70b-versatile` | Model used for polishing (varies by tier) |
| Auto-paste | `true` | Automatically paste into active app |

## Project structure

```
talx/
├── src/                          # React frontend (widget UI)
│   ├── components/               # React components
│   │   ├── Widget.tsx            # Main widget (orchestrates states)
│   │   ├── IdlePill.tsx          # Minimal idle state
│   │   ├── RecordingCard.tsx     # Expanded recording view
│   │   └── ResultCard.tsx        # Polished result with actions
│   ├── hooks/                    # Custom React hooks
│   ├── services/                 # API integrations (Soniox, LLM)
│   ├── App.tsx
│   └── main.tsx
├── src-tauri/                    # Rust backend (system integration)
│   ├── src/
│   │   ├── lib.rs                # Tauri commands
│   │   ├── hotkey.rs             # Global hotkey registration
│   │   ├── audio.rs              # Microphone capture
│   │   └── paste.rs              # Text injection into active app
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                         # Documentation
└── README.md
```

## Roadmap

- [x] Core dictation pipeline (record → transcribe → polish → paste)
- [x] Settings UI with API key management
- [ ] Voice Activity Detection (client-side, no cloud for silence detection)
- [ ] Session history (local, via IndexedDB)
- [ ] Custom dictionaries and snippets
- [ ] Windows support
- [ ] Linux support

## Contributing

Contributions are welcome! See [docs/contributing.md](docs/contributing.md) for the full guide.

**Good first issues** are labeled [`good first issue`](https://github.com/paarangat/talx/labels/good%20first%20issue) — great starting points for new contributors.

### Where to contribute

| Area | Directory | Requires |
|------|-----------|----------|
| Widget UI, animations, settings | `src/` | React + TypeScript |
| Hotkeys, audio, paste, performance | `src-tauri/` | Rust |
| Guides, translations, setup docs | `docs/` | Markdown |

## Security

- API keys are stored in the OS keychain, never in plaintext
- Audio is sent directly to the ASR provider, never stored locally
- Transcripts exist only in memory during processing
- No telemetry, no analytics, no phone-home

## License

[MIT](LICENSE) — free to use, modify, and distribute.

## Acknowledgments

Inspired by [Wispr Flow](https://wisprflow.ai). Built with [Tauri](https://v2.tauri.app/), [Groq](https://groq.com/), [Soniox](https://soniox.com/), and [OpenAI](https://openai.com/).
