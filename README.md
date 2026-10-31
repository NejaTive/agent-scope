# Agent Scope

<p align="center">
  <img src="docs/assets/app-icon.png" width="96" height="96" alt="Agent Scope">
</p>

<h3 align="center">All your AI tooling config — in one place.</h3>

<p align="center">
  A macOS app for viewing, analyzing, and managing configuration for AI development tools.
</p>

<p align="center">
  <a><b>English</b></a> ·
  <a href="docs/ru/README.md">Русский</a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#features">Features</a> ·
  <a href="https://github.com/NejaTive/agent-scope/releases">Releases</a>
</p>

<p align="center">
  <img src="docs/assets/overview.png" width="800" alt="Overview">
</p>

---

## Why Agent Scope?

Claude Code, Cursor, and other AI tools rely on more and more config files: skills, rules, agents, hooks, MCP servers, plugins, and memory.

Some settings live in your home directory, others inside individual projects. Some override global defaults; others duplicate what you already have.

The more AI tools and projects you use, the harder it is to keep track of everything by hand.

**Agent Scope brings those configurations together in one UI.**

The app scans global and project settings, shows their structure, helps you find conflicts, and lets you safely edit supported files.

No more digging through dozens of hidden directories by hand.

## Install

**Requirements:** macOS 11 (Big Sur) or newer · Apple Silicon

The easiest way to install Agent Scope is with Homebrew:

```bash
brew tap nejative/agent-scope
brew install --cask agent-scope
```

If Homebrew reports that the source is untrusted (`untrusted tap` or `untrusted cask`), run:

```bash
brew trust --cask nejative/agent-scope/agent-scope
```

Then install again:

```bash
brew install --cask agent-scope
```

## Features

### Unified AI tooling overview

See how your AI environment is structured:

- Skills and rules.
- Agents and hooks.
- MCP servers and plugins.
- Memory and other supported artifacts.

Global and project settings are available in one place.

### Comfortable config browsing

Stop hopping between hidden directories and text editors.

- Browse Markdown and JSON files.
- Search for the configs you need.
- See where each artifact comes from.
- Compare global settings with per-project ones.

### Conflict and duplicate detection

Different AI tools and projects can share overlapping settings.

Agent Scope helps you spot:

- Duplicate artifact names.
- MCP configuration mismatches.
- Setting overrides.
- Potential inconsistencies between global and project configs.

### Change history

Keep an eye on how your AI environment evolves.

The change feed shows recently created, modified, and deleted files.

That makes it easy to see what changed after a tool update, a new plugin, or a project tweak.

### Editing under your control

Agent Scope is primarily for viewing configs, but sometimes you need a quick edit.

With editing enabled you can:

- Change the contents of supported settings.
- Rename existing settings within the same directory.
- Delete existing settings.

**Editing is off by default.**

### Local and secure

Agent Scope is a local macOS app.

- Config scanning runs locally.
- No AI service or cloud account is required.
- Secrets are masked when viewing files.
- Full contents are available only in explicitly enabled edit mode.
- The app does not execute configuration files.

## Supported tools

| Tool        | Support                         |
| ----------- | ------------------------------- |
| Claude Code | Global and project configs      |
| Cursor      | Global and project configs      |

**Upgrade** to the latest version:

```bash
brew upgrade --cask agent-scope
```

You can also download a DMG from [GitHub Releases](https://github.com/NejaTive/agent-scope/releases).

**First launch:** if macOS shows a Gatekeeper warning, open the app via Right-click → **Open**.

## Limitations

Agent Scope focuses on viewing and safely managing existing configurations.

It is not meant for:

- Creating new skills, agents, rules, and other artifacts from scratch.
- Running hooks, MCP servers, or shell commands from config files.
- Changing credentials or files outside the allowlist.
- Running on Intel Macs.

## Tech stack

Agent Scope is built with a modern native stack:

- **Tauri 2** — lightweight desktop shell.
- **Rust** — scanning, config processing, and filesystem work.
- **Vue 3** — user interface.
- **TypeScript** — frontend typing.
- **Tailwind CSS** — UI styling.

## Development

Clone the repository and install dependencies:

```bash
npm install
```

Run the app in development mode:

```bash
npm run tauri:dev
```

Run Rust tests:

```bash
cd src-tauri
cargo test
```

Build the frontend:

```bash
npm run build
```

Build the macOS app:

```bash
npm run tauri:build
```

Built DMGs and other artifacts are under:

```text
src-tauri/target/release/bundle/
```

## Documentation

- [Claude Code spec](docs/spec/claude-code.md).
- [Cursor spec](docs/spec/cursor.md).

## License

[MIT](LICENSE) © Alexandr Sharov
