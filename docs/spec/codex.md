# OpenAI Codex configuration map (Agent Scope)

**Spec cut date:** 2026-11-01  
**Sources:** [Configuration reference](https://developers.openai.com/codex/config-reference), [Environment variables](https://developers.openai.com/codex/config), [AGENTS.md](https://developers.openai.com/codex/guides/agents-md), [Skills](https://developers.openai.com/codex/skills), [MCP](https://developers.openai.com/codex/mcp), [Hooks](https://developers.openai.com/codex/hooks)

Agent Scope treats OpenAI Codex docs as source of truth. When formats drift, update this file and collectors together. Prefer docs over guesses; mark confidence when Agent Scope does not emulate Codex runtime (trust gating, config merge).

## Config home

| Scope | Location | Notes |
|-------|----------|-------|
| User | `CODEX_HOME` or `~/.codex` | Root for config, auth, logs, sessions, legacy skills. If `CODEX_HOME` is set, the directory must already exist (per docs). |
| Project | `<root>/.codex/` | Project-scoped overrides. Docs: Codex loads project config **only when the project is trusted** — Agent Scope inventories files on disk; trust gating is **not** modeled (`inferred` / `unknown`). |
| Admin skills | `/etc/codex/skills` | Machine-wide skill roots when present and readable. |

### Never read

- `auth.json` and other credential / secret files under `CODEX_HOME`
- Session logs / SQLite state (out of scope for config inventory)

## Settings (`config.toml`)

| Scope | Path |
|-------|------|
| User | `$CODEX_HOME/config.toml` (default `~/.codex/config.toml`) |
| Project | `.codex/config.toml` (and nested project layers only as files found on disk) |

Agent Scope shows TOML as text (`language` = `toml`). Full key merge / trust-gated project load → **inferred** / **unknown**; we do not claim parity with a live Codex session.

MCP servers are configured as `[mcp_servers.<name>]` tables inside `config.toml` (user and trusted project). Inventory: the settings file itself; per-server `McpServerEntry` extraction is optional and must not invent keys.

## Instructions (`AGENTS.md`)

Per docs:

1. **Global:** In Codex home, prefer `AGENTS.override.md` if present; else `AGENTS.md`. Only the first non-empty file at that level is used by Codex.
2. **Project:** From repo root down to CWD, per directory: `AGENTS.override.md`, then `AGENTS.md`, then optional fallback names from config. At most one file per directory in Codex runtime.

Agent Scope inventories present files; it does not simulate the walk-to-CWD merge order beyond listing known paths.

### Shared with Cursor / Claude

Bare project `AGENTS.md` and user/project `.agents/skills/**` are also used by Cursor (and optionally Claude). See [Ownership & dedup](#ownership--dedup).

## Skills

A skill is a directory containing `SKILL.md` (YAML frontmatter + body).

| Scope | Location | Notes |
|-------|----------|-------|
| Repo | `.agents/skills/` (CWD up to repo root) | Shared layout with Cursor |
| User | `~/.agents/skills/` | Shared with Cursor |
| Admin | `/etc/codex/skills` | If readable |
| System | Bundled with Codex | Not on disk for Agent Scope |
| Legacy | `$CODEX_HOME/skills` / `~/.codex/skills` | Older layout; inventory if present, mark **legacy** in metadata |

Compatibility: Cursor docs also load `.codex/skills` — Agent Scope attributes paths under `.codex/` / `CODEX_HOME` to **Codex**, not Cursor.

## Hooks

Discovered next to active config layers:

- `hooks.json` beside the layer (`~/.codex/hooks.json`, `.codex/hooks.json`)
- Inline `[hooks]` / `[[hooks.*]]` inside `config.toml`

Agent Scope inventories `hooks.json` as files (similar to Cursor). Inline TOML hooks stay inside the settings artifact (no separate execution; never run hooks).

Project hooks load in Codex only when the project is trusted → confidence **inferred**.

## MCP

Stored in `config.toml` (`[mcp_servers.<name>]`), not a separate `.mcp.json` like Claude. See MCP docs. No invented alternate paths.

## Ownership & dedup

One filesystem path → one artifact. Product tag / path gating:

| Path pattern | `PathProduct` / artifact `product` |
|--------------|--------------------------------------|
| Under `.claude` / Claude home | Claude |
| Under `.cursor` / Cursor home | Cursor |
| Under `.codex` / `CODEX_HOME` | Codex |
| `.agents/skills/**`, bare `AGENTS.md` / `AGENTS.override.md` | **Shared** for allowlist/watch (enabled if any of Claude, Cursor, Codex is tracked) |

**Scan order:** Claude → Cursor → Codex, with a shared `seen` path set. First collector to claim a path wins the artifact `product` tag (preserves existing Claude/Cursor UX). When only Codex is tracked, Codex owns shared `AGENTS.md` / `.agents/skills`.

## Preferences

- Pref key: `trackCodex` (default **`true`**, same as Claude/Cursor; presence indicator shows whether `CODEX_HOME` / `~/.codex` exists).
- Disabled Codex is not scanned, watched, or shown.
- At least one of `trackClaude` / `trackCursor` / `trackCodex` must remain enabled.

## Confidence (Effective Configuration)

| Level | Meaning |
|-------|---------|
| `computed` | Documented file present and inventoried |
| `inferred` | Likely (trust-gated project config, merge order) |
| `unknown` | Runtime-only (approvals, live trust store, session state) |

## Out of scope (this product map)

- Emulating Codex trust / sandbox / approvals
- Reading `auth.json`, session DBs, logs
- Full Windsurf / Copilot / Aider / Cline scanners (stubs/TODOs elsewhere)
- Bundled system skills shipped inside the Codex binary
