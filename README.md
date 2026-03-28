# Talx

**Open-source voice-to-text dictation that works everywhere on your desktop.**

Speak naturally in any app — Talx transcribes your speech in real-time, polishes it with AI, and pastes the result right where your cursor is. Supports English, Hindi, and mixed-language (code-switching) dictation.

Think [Wispr Flow](https://wisprflow.ai), but open-source, self-hosted with your own API keys, and privacy-first.

## Features

- 🎙️ **System-wide dictation** — works in any text field, any app
- ⚡ **Real-time streaming** — see your words appear as you speak
- 🧠 **AI polishing** — removes filler words, fixes grammar, matches context
- 🌍 **Multilingual** — English, Hindi, and code-switching (Hinglish) out of the box
- 🔒 **Privacy-first** — your API keys, your data, no middleman
- 💰 **Free tier** — Groq-powered ASR and LLM at no cost (generous free API keys)
- 💰 **Cloud tier** — bring your own Soniox + OpenAI keys for streaming and higher accuracy
- 🪶 **Lightweight** — ~8MB bundle, ~50MB RAM (vs 150MB+ for Electron apps)

## How it works

```
[You speak] → [ASR: Groq or Soniox] → [Raw transcript] → [LLM: Groq or OpenAI] → [Polished text] → [Pasted into active app]
     ↑                                                                                                          ↓
  Hold ⌥Space                                                                                           Release ⌥Space
```

1. Hold the hotkey (default: `⌥ Space`)
2. Speak naturally — the floating widget shows your words streaming in
3. Release the hotkey — AI cleans up the transcript
4. Polished text is auto-pasted into whatever app you're using

## Prerequisites

- [Rust](https://rustup.rs/) (for the Tauri backend)
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- [Groq API key](https://console.groq.com/) (free tier — ASR + LLM)
- [Soniox API key](https://console.soniox.com/) (cloud tier — streaming ASR)
- [OpenAI API key](https://platform.openai.com/) (cloud tier — AI polishing)

### macOS additional requirements

```bash
xcode-select --install
```

### Linux additional requirements (Ubuntu/Debian)

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

### Windows additional requirements

- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- WebView2 (pre-installed on Windows 10/11)

## Quick start

```bash
# Clone the repo
git clone https://github.com/YOUR_USERNAME/talx.git
cd talx

# Install dependencies
pnpm install

# Set up your API keys (first time only)
cp .env.example .env
# Edit .env with your API keys (Groq for free tier, Soniox + OpenAI for cloud tier)

# Run in development mode
pnpm tauri dev
```

## Configuration

On first launch, Talx will prompt you to enter your API keys. They're stored securely in your OS keychain.

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
│   │   ├── useRecording.ts       # Audio recording state machine
│   │   └── useSettings.ts       # User preferences
│   ├── services/                 # API integrations
│   │   ├── soniox.ts             # Soniox WebSocket client
│   │   └── llm.ts                # Anthropic API client
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
│   ├── architecture.md           # Technical design decisions
│   └── contributing.md           # How to contribute
├── CLAUDE.md                     # Claude Code project context
├── LICENSE                       # MIT License
└── README.md
```

## Cost comparison

At typical usage (~30 min/day of dictation):

| | Wispr Flow Pro | Talx (self-hosted) |
|---|---|---|
| Monthly cost | $12/month | Free (Groq) / ~$2-3 (your own cloud keys) |
| Annual cost | $144/year | Free (Groq) / ~$24-36 (your own cloud keys) |
| Data privacy | Cloud (their servers) | Your own API keys, always |
| Customization | None | Full control, open source |

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

## Roadmap

- [x] Core dictation pipeline (record → transcribe → polish → paste)
- [ ] Settings UI with API key management
- [ ] Voice Activity Detection (client-side, no cloud for silence detection)
- [ ] Session history (local, via IndexedDB)
- [ ] Custom dictionaries and snippets
- [ ] Windows support
- [ ] Linux support

## Contributing

We welcome contributions! See [docs/contributing.md](docs/contributing.md) for guidelines.

**Good first issues** are labeled [`good first issue`](https://github.com/YOUR_USERNAME/talx/labels/good%20first%20issue) — these are great starting points for new contributors.

## License

MIT — see [LICENSE](LICENSE) for details.

## Acknowledgments

Inspired by [Wispr Flow](https://wisprflow.ai). Built with [Tauri](https://v2.tauri.app/), [Groq](https://groq.com/), [Soniox](https://soniox.com/), and [OpenAI](https://openai.com/).
