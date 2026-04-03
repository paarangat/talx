<p align="center">
  <img src="src-tauri/icons/source-logo.png" alt="Talx Logo" width="120" />
</p>

<h1 align="center">Talx</h1>

<p align="center">
  <strong>Open-source voice-to-text dictation for macOS. Speak anywhere, get polished text.</strong>
</p>

<p align="center">
  <a href="#features">Features</a> &middot;
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="#configuration">Configuration</a> &middot;
  <a href="#contributing">Contributing</a> &middot;
  <a href="#roadmap">Roadmap</a>
</p>

---

Talx is a floating dictation widget for macOS. Hold a hotkey, speak, and it transcribes your speech, polishes it with an LLM, and pastes the result into whatever app you're using. You bring your own API keys — nothing is routed through our servers.

## Features

- **System-wide dictation** — works in any text field, any app
- **Real-time streaming** — see words appear as you speak (with Soniox)
- **AI polishing** — removes filler words, fixes grammar, formats text
- **Two ASR providers** — Groq Whisper (free, batch) or Soniox (paid, streaming)
- **Two LLM providers** — Groq (free) or OpenAI (paid)
- **Model selection** — choose specific models per provider
- **Session history** — transcriptions saved locally in SQLite
- **Dashboard** — usage stats and recent transcriptions
- **Configurable hotkey** — default `⌥ Space`, customizable in settings

## How it works

```
[You speak] → [ASR: Groq or Soniox] → [Raw transcript] → [LLM: Groq or OpenAI] → [Polished text] → [Pasted into active app]
     ↑                                                                                                          ↓
  Hold ⌥Space                                                                                           Release ⌥Space
```

1. **Hold** the hotkey (default: `⌥ Space`)
2. **Speak** — the floating widget shows your transcript
3. **Release** — the LLM cleans up the transcript
4. **Done** — polished text is auto-pasted into the active app

## Tech stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | [Tauri v2](https://v2.tauri.app/) |
| Frontend | React + TypeScript + Vite |
| Transcription (free) | [Groq Whisper](https://console.groq.com/) |
| Transcription (streaming) | [Soniox API](https://soniox.com/) |
| LLM polishing (free) | [Groq LLM](https://console.groq.com/) |
| LLM polishing (paid) | [OpenAI API](https://platform.openai.com/) |
| Backend | Rust |

## Prerequisites

- macOS
- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- At least one set of API keys:
  - **Free**: [Groq API key](https://console.groq.com/) — handles both ASR and LLM
  - **Streaming**: [Soniox API key](https://console.soniox.com/) — real-time transcription
  - **Paid LLM**: [OpenAI API key](https://platform.openai.com/) — alternative polishing

```bash
xcode-select --install  # if you don't have Xcode CLI tools
```

## Quick start

```bash
git clone https://github.com/paarangat/talx.git
cd talx
pnpm install
pnpm tauri dev
```

On first launch, open settings to enter your API keys. They're stored in a local SQLite database.

## Configuration

All settings are configurable through the settings UI:

| Setting | Default | Description |
|---------|---------|-------------|
| Hotkey | `⌥ Space` | Hold to record, release to process |
| ASR provider | Groq | Groq (batch) or Soniox (streaming) |
| LLM provider | Groq | Groq or OpenAI |
| ASR/LLM model | Provider default | Selectable per provider |
| Auto-paste | On | Paste polished text into active app |

## Project structure

```
talx/
├── src/                          # React frontend
│   ├── App.tsx                   # Root component, window routing via query params
│   ├── Dashboard.tsx             # Dashboard view
│   ├── Settings.tsx              # Settings with tab navigation
│   ├── components/
│   │   ├── Widget.tsx            # Floating widget (idle/recording/polishing/success)
│   │   ├── dashboard/            # Stats, provider status, recent transcriptions
│   │   └── settings/             # General, API keys, appearance, about tabs
│   ├── hooks/                    # useDrag, useSessionStore
│   ├── stores/                   # Session stats (words, duration, count)
│   └── styles/                   # CSS tokens, animations
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── lib.rs                # Tauri commands, app state, hotkeys
│   │   ├── audio.rs              # cpal-based microphone capture
│   │   ├── db.rs                 # SQLite (transcriptions + settings)
│   │   ├── asr/                  # Groq Whisper, Soniox WebSocket
│   │   └── llm/                  # Groq LLM, OpenAI
│   └── tauri.conf.json
└── docs/                         # Architecture, design system, contributing guide
```

## Roadmap

- [x] Core dictation pipeline (record → transcribe → polish → paste)
- [x] Settings UI with API key management
- [x] Session history in SQLite
- [x] Dashboard with usage stats and recent transcriptions
- [x] Model selection per provider
- [ ] Voice activity detection
- [ ] Windows and Linux support

## Contributing

See [docs/contributing.md](docs/contributing.md).

## Security

- API keys stored locally in SQLite on your machine
- Audio sent directly to the ASR provider you choose
- Transcripts stored locally only
- No telemetry, no analytics

## License

[MIT](LICENSE)
