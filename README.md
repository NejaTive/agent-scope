# Agent Scope

Local macOS desktop app for **viewing and analyzing** Claude Code configuration — global and project scopes — without modifying files.

Built with **Tauri 2**, **Rust**, **Vue 3**, TypeScript, Tailwind.

## What it does

- Scans `~/.claude` (or `CLAUDE_CONFIG_DIR`) and managed macOS paths
- Discovers projects under user-selected roots (or add a project manually)
- Explores Skills, Commands, Rules, Agents, Hooks, MCP, Plugins, Settings, Memory
- Shows Effective Configuration with explicit confidence (`computed` / `inferred` / `unknown`)
- Detects conflicts (duplicate names, MCP mismatches, setting overrides)
- Watches config files and lists recent changes

## What it does **not** do

- Does not create, edit, or delete Claude Code config (read-only UI)
- Does not run hooks, MCP servers, or shell commands from configs
- Does not display credential file contents or unmasked secrets by default

## Requirements

- macOS 11+ (Apple Silicon; Intel via cross-compile / universal build)
- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) stable
- Xcode Command Line Tools

## Develop

```bash
npm install
npm run tauri:dev
```

Frontend only (no IPC):

```bash
npm run dev
```

## Test

```bash
cd src-tauri && cargo test
npm run build   # vue-tsc + vite
```

## Build macOS app / DMG

```bash
npm run tauri:build
```

Artifacts land under `src-tauri/target/release/bundle/` (`.app`, `.dmg`).

### Notes

- Grant folder access when picking scan roots / projects (minimal TCC; no Full Disk Access required for user-chosen paths).
- First scan of large trees may take a few seconds; excluded dirs: `node_modules`, `target`, `dist`, `build`, `.git`, etc.

## App data

SQLite preferences / scan roots / change history:

`~/Library/Application Support/com.agentscope.desktop/agent-scope.sqlite`

Claude configs stay in their original files.

## Spec

See [docs/spec/claude-code.md](docs/spec/claude-code.md).

## License

Private / pet project unless otherwise stated.
