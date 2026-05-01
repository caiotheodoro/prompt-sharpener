# Prompt Sharpener

Prompt Sharpener is a tiny desktop overlay for turning rough AI prompts into precise instructions before you send them.

It sits in the background, waits for a global shortcut, reads the text you are already writing in Cursor, Claude Code, Codex, a terminal, or an editor, then opens a small review box with a sharpened version of the prompt. Accept it to replace the original text, or discard it and keep writing.

The goal is simple: keep your flow, but make every prompt clearer before it reaches the model.

## Why It Exists

Coding assistants are sensitive to prompt quality. A prompt like:

```text
fix this
```

usually makes the model guess. Prompt Sharpener rewrites it into something more explicit: task, scope, constraints, output format, and success criteria. It is especially useful when you are moving quickly inside agent tools and do not want to stop to manually polish every instruction.

## Capabilities

- Global shortcut overlay for sharpening selected text.
- Works with coding assistants, terminals, and normal editors.
- Accept/discard flow so you can review before replacing anything.
- Terminal mode for Claude Code, Codex, shells, and other prompt inputs where selection replacement is unreliable.
- Editor mode for Cursor, VS Code, and text fields where normal paste replaces selected text.
- Resizable result window for long prompts.
- Tray menu and settings window.
- Cloud and local model providers:
  - Anthropic Claude
  - OpenAI
  - Google Gemini
  - Ollama running locally
- Local-first option with Ollama and Qwen3 models.
- API keys stored locally through Tauri's app store.

## How It Works

1. Select or focus the prompt text you want to improve.
2. Press the global shortcut.
3. Prompt Sharpener copies the current text and sends it to your selected model provider.
4. A compact overlay shows the improved prompt.
5. Accept to replace the original text, or discard to leave it unchanged.

Default shortcuts:

- Windows and Linux: `Ctrl+Alt+P`
- macOS: `Cmd+Alt+P`

## Providers

| Provider | Models exposed by default | Best for |
| --- | --- | --- |
| Anthropic | Haiku 4.5, Sonnet 4.6 | Strong instruction following and prompt rewriting |
| OpenAI | GPT-5.4 Nano, GPT-5.4 Mini | Low-latency cloud sharpening |
| Google Gemini | Gemini 2.5 Flash-Lite, Gemini 3 Flash | Fast cloud sharpening with generous model options |
| Ollama | Qwen3 1.7B, Qwen3 4B, Qwen3 8B | Local, private, no API key required |

## Ollama Local Mode

Ollama mode talks to the local OpenAI-compatible endpoint:

```text
http://localhost:11434/v1/chat/completions
```

Install Ollama from:

```text
https://ollama.com
```

Then pull the default model:

```bash
ollama pull qwen3:1.7b
```

Recommended local lineup:

| Model | Use case |
| --- | --- |
| `qwen3:1.7b` | Default local model, very fast, runs on most machines |
| `qwen3:4b` | Better quality if you have more RAM |
| `qwen3:8b` | Best local quality among the built-in presets |

The settings page includes a status check button that verifies whether Ollama is reachable.

## Accept Target Modes

Prompt Sharpener supports two replacement strategies:

- `Terminal`: clears the current prompt input and pastes the improved prompt. Use this for Claude Code, Codex, shells, and terminal UIs.
- `Editor`: pastes normally so the selected text is replaced by the target app. Use this for Cursor, VS Code, or regular text editors.

## Installation

Download the latest release for your platform from GitHub Releases:

```text
https://github.com/caiotheodoro/prompt-sharpener/releases
```

Release artifacts are built for:

- Windows
- macOS
- Linux

Notes:

- macOS builds are unsigned unless a signed/notarized build is provided in a future release.
- Windows builds are unsigned unless a code-signing certificate is added later.
- Linux users may need system WebKit/AppIndicator libraries depending on their distribution.

## Linux Requirements

For development or local builds on Ubuntu/Debian-style systems:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  curl \
  wget \
  file \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libxdo-dev \
  libssl-dev
```

`xdotool` is optional. The app uses it only as a Linux fallback for cursor-based overlay placement:

```bash
sudo apt-get install -y xdotool
```

## Development

Install dependencies:

```bash
pnpm install
```

Run the frontend dev server:

```bash
pnpm dev
```

Run Tauri in development:

```bash
pnpm tauri dev
```

Build the frontend:

```bash
pnpm build
```

Build the desktop app:

```bash
pnpm tauri build
```

Windows cross-build from WSL, as used during development:

```bash
CI=false npx tauri build --target x86_64-pc-windows-gnu --no-bundle
```

## Release Workflow

This repository is configured so:

- Every push and pull request builds the app on Windows, macOS, and Linux and uploads artifacts.
- Every tag matching `v*` builds native platform bundles and attaches them to a GitHub Release.

To publish a release:

```bash
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

## Security

- API keys are stored locally through Tauri's app store.
- Do not commit `.env`, `.env.local`, `.cursor`, local debug logs, or Tauri target directories.
- Ollama mode keeps prompt sharpening local, assuming your selected model is installed and running locally.

## Status

Prompt Sharpener is early, practical software. It already handles the main loop: select, sharpen, review, accept. The next step is making releases smoother and expanding platform-specific polish.
