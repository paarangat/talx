<p align="center">
  <img src="src-tauri/icons/source-logo.png" alt="Talx Logo" width="120" />
</p>

<h1 align="center">Talx</h1>

<p align="center">
  <strong>Open-source voice-to-text dictation for macOS & Windows. Speak anywhere, get polished text.</strong>
</p>

<p align="center">
  <a href="#features">Features</a> &middot;
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="#configuration">Configuration</a> &middot;
  <a href="#why-free">Why Free?</a> &middot;
  <a href="#contributing">Contributing</a> &middot;
  <a href="#roadmap">Roadmap</a>
</p>

---

Talx is a floating dictation widget for macOS & Windows. Hold a hotkey, speak, and it transcribes your speech, polishes it with an LLM, and pastes the result into whatever app you're using. You bring your own API keys — nothing is routed through our servers.

## Features

- **System-wide dictation** — works in any text field, any app
- **Real-time streaming** — see words appear as you speak (with Soniox)
- **AI polishing** — removes filler words, fixes grammar, formats text
- **Two ASR providers** — Groq Whisper (free, batch) or Soniox (paid, streaming)
- **Two LLM providers** — Groq (free) or OpenAI (paid)
- **Model selection** — choose specific models per provider
- **Voice privacy** — pitch-shifts audio (+4 semitones) before cloud upload so your real voice never leaves your machine
- **Mouse button hotkey** (Windows) — trigger recording with a mouse side button or middle click
- **Session history** — transcriptions saved locally in SQLite
- **Dashboard quit button** — fully exit Talx from the dashboard sidebar
- **Dashboard** — usage stats and recent transcriptions
- **Configurable hotkey** — default `⌥ Space` (macOS) / `Ctrl+Shift+Space` (Windows), customizable in settings

## How it works

```
[You speak] → [ASR: Groq or Soniox] → [Raw transcript] → [LLM: Groq or OpenAI] → [Polished text] → [Pasted into active app]
     ↑                                                                                                          ↓
  Hold ⌥Space                                                                                           Release ⌥Space
```

1. **Hold** the hotkey (macOS: `⌥ Space`, Windows: `Ctrl+Shift+Space`)
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

### macOS

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- Xcode CLI tools:

```bash
xcode-select --install
```

### Windows

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) with "Desktop development with C++" workload

At least one set of API keys:

- **Free**: [Groq API key](https://console.groq.com/) — handles both ASR and LLM
- **Streaming**: [Soniox API key](https://console.soniox.com/) — real-time transcription
- **Paid LLM**: [OpenAI API key](https://platform.openai.com/) — alternative polishing

## Quick start

```bash
git clone https://github.com/paarangat/talx.git
cd talx
pnpm install
pnpm tauri dev
```

On first launch, open settings to enter your API keys. They're stored in a local SQLite database.

### Building a standalone desktop app

```bash
pnpm tauri build
```

This produces a standalone `.exe` (Windows) or `.app` (macOS) in `src-tauri/target/release/` — no terminal or dev tools needed to run it. An installer is also generated at `src-tauri/target/release/bundle/`.

## Configuration

All settings are configurable through the settings UI:

| Setting | Default | Description |
|---------|---------|-------------|
| Hotkey | `⌥ Space` (macOS) / `Ctrl+Shift+Space` (Windows) | Hold to record, release to process |
| Mouse hotkey | None (Windows only) | Mouse side button or middle click to record |
| ASR provider | Groq | Groq (batch) or Soniox (streaming) |
| LLM provider | Groq | Groq or OpenAI |
| ASR/LLM model | Provider default | Selectable per provider |
| Auto-paste | On | Paste polished text into active app |
| Voice privacy | On | Pitch-shift audio +4 semitones before cloud upload |

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
│   │   ├── pitch_shift.rs        # Phase vocoder for voice privacy
│   │   ├── mouse_hook.rs         # Low-level mouse hook (Windows)
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
- [x] Windows support

## Why free?

Talx uses Groq's free API tier for both transcription and LLM polishing. Here's how that compares to Wispr Flow, the leading commercial alternative:

| | Wispr Flow Free | Wispr Flow Pro | Talx (Groq free tier) |
|---|---|---|---|
| **Cost** | $0 | $12–15/mo | $0 |
| **Daily word budget** | ~285 words (2K/week) | Unlimited | ~72,000 words (8 hrs audio) |
| **Weekly words** | 2,000 | Unlimited | ~504,000 |
| **Streaming transcription** | Yes | Yes | Batch only (streaming via Soniox, paid) |
| **AI polishing** | Pro only | Yes | Yes |
| **Open source** | No | No | Yes |

Groq's free tier provides 8 hours of audio transcription per day (28,800 seconds) and 1,000 LLM polishing calls — roughly **250x the capacity** of Wispr Flow's free plan. The only trade-off is per-minute rate limits (20 STT requests/min) which don't affect normal dictation.

## Why I built this

I was using [Wispr Flow](https://wisprflow.com/) pretty much every day for dictation — it genuinely made my workflow faster, and I got hooked while on their free Pro trial. Once the trial ended, I was down to 2,000 words per week on the free tier, which if you actually use dictation daily, is nothing.

So I built an alternative that technically has no limit for an individual user. Groq's free API tier gives you up to 8 hours of audio transcription per day — roughly 250x what Wispr Flow offers for free. Same idea: speak, get polished text, paste it wherever your cursor is. No subscription required.

## Security

- API keys stored locally in SQLite on your machine
- Audio sent directly to the ASR provider you choose
- Voice privacy mode pitch-shifts audio (+4 semitones) before upload so your real voice never leaves your machine (enabled by default)
- Transcripts stored locally only
- No telemetry, no analytics

## License

[MIT](LICENSE)
