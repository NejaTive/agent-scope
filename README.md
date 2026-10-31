# Agent Scope

<p align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="Agent Scope">
</p>

<p align="center">
  <strong>See every Claude Code &amp; Cursor config in one place.</strong><br>
  Local macOS app — scan, inspect conflicts, optionally edit allowlisted files.
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="docs/user-guide.md">User guide</a> ·
  <a href="docs/ru/README.md">Русский</a> ·
  <a href="https://github.com/NejaTive/agent-scope/releases">Releases</a>
</p>

<p align="center">
  <img alt="macOS" src="https://img.shields.io/badge/macOS-11%2B%20Apple%20Silicon-111?style=flat-square">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-0a7?style=flat-square">
  <img alt="Version" src="https://img.shields.io/github/v/release/NejaTive/agent-scope?style=flat-square&label=release">
  <img alt="Homebrew" src="https://img.shields.io/badge/homebrew-cask-fbb?style=flat-square">
</p>

<!-- Screenshots: add PNGs under docs/assets/ (see docs/assets/README.md), then uncomment:
<p align="center">
  <img src="docs/assets/dashboard.png" width="800" alt="Dashboard">
</p>
-->

## Why Agent Scope

Claude Code and Cursor both scatter skills, rules, agents, hooks, MCP servers, and memory across global and project trees. Agent Scope gives you a single local UI to:

- **Inventory** everything under `~/.claude` / `CLAUDE_CONFIG_DIR`, `~/.cursor`, and the projects you add
- **Spot conflicts** — duplicate names, MCP mismatches, setting overrides
- **Track changes** after each scan
- **Edit safely (opt-in)** — rename / delete / save only existing allowlisted files; credentials and managed paths stay untouched

Stack: **Tauri 2**, **Rust**, **Vue 3**, TypeScript, Tailwind.

## Install

**Apple Silicon only** · macOS 11 (Big Sur)+

```bash
brew tap nejative/agent-scope
brew install --cask agent-scope
```

Upgrade: `brew upgrade --cask agent-scope`

DMG downloads: [Releases](https://github.com/NejaTive/agent-scope/releases). First launch may need Right-click → **Open** if Gatekeeper warns (unsigned builds) — details in the [user guide](docs/user-guide.md#gatekeeper).

## Features

| | |
|--|--|
| **Overview dashboard** | Counts for projects, skills, rules, agents, hooks, MCP, plugins, memory |
| **Global + project scope** | Browse user-level and per-repo artifacts side by side |
| **Artifact browser** | Markdown / JSON viewers with search |
| **Conflict detection** | Duplicate names, MCP drift, setting overrides |
| **Change feed** | Recent created / modified / deleted config files |
| **Opt-in editing** | Raw edit, same-folder rename, single-file delete — off by default |
| **Secret masking** | Masked in view mode; unmasked only while editing |
| **Update check** | Notices when a newer GitHub Release exists |

**Does not:** create artifacts from scratch, run hooks/MCP/shell from configs, write credentials or paths outside the allowlist, or support Intel Macs.

## Quick start (from source)

```bash
npm install
npm run tauri:dev
```

Tests: `cd src-tauri && cargo test` · frontend typecheck/build: `npm run build`  
Release DMG: `npm run tauri:build` → `src-tauri/target/release/bundle/`

Full setup, editing rules, and data paths: **[docs/user-guide.md](docs/user-guide.md)**.

## Documentation

| Doc | |
|-----|--|
| [User guide](docs/user-guide.md) | Install, first scan, editing |
| [Homebrew tap](docs/homebrew.md) | Maintainer release checklist |
| [Claude Code map](docs/spec/claude-code.md) · [Cursor map](docs/spec/cursor.md) | Config layout specs |
| [Русская документация](docs/ru/README.md) | README и гайды на русском |

## License

[MIT](LICENSE) © Alexandr Sharov
