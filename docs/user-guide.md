# User guide

> Русская версия: [ru/user-guide.md](ru/user-guide.md)

Agent Scope is a local macOS app for **inspecting and (optionally) editing** Claude Code and Cursor configuration — global and project scopes.

## Install

**Apple Silicon only** (Intel is not supported). macOS 11+.

```bash
brew tap nejative/agent-scope
brew install --cask agent-scope
# upgrade
brew upgrade --cask agent-scope
```

Or download the `.dmg` from [GitHub Releases](https://github.com/NejaTive/agent-scope/releases).

### Gatekeeper

Unsigned builds may be blocked on first launch. Right-click the app → **Open**, or clear quarantine:

```bash
xattr -dr com.apple.quarantine "/Applications/Agent Scope.app"
```

## First launch

1. Grant folder access when choosing scan roots / projects (minimal TCC; Full Disk Access is not required for paths you pick yourself).
2. Wait for the first scan — large trees can take a few seconds. Excluded by default: `node_modules`, `target`, `dist`, `build`, `.git`, and similar.
3. Use the sidebar: Overview, Global, Projects, Skills, Rules, Agents, Hooks, MCP, Plugins, Permissions, Settings, Memory.

## What you get

| Area | Examples |
|------|----------|
| Skills & commands | `SKILL.md`, slash commands |
| Rules & agents | project/user rules, agent defs |
| Hooks & MCP | hook configs, MCP server entries |
| Plugins & permissions | plugin lists, permission settings |
| Settings | scoped settings + diff (view-only) |
| Memory | `AGENTS.md`, `CLAUDE.md`, memory files |

Conflicts called out on the dashboard include duplicate names, MCP mismatches, and setting overrides. Recent file changes are tracked after scans.

## Editing (opt-in)

Editing is **off** by default.

1. Open app settings → enable **Allow editing configuration files** (`allowEditing`).
2. Open an artifact → **Edit** → change the raw file → **Save**; or **Rename** / **Delete**.
3. **Rename** — new name in the **same folder** only; destination must not exist and must stay on the allowlist (e.g. `CLAUDE.md` → `FOO.md` in the project root is rejected).
4. **Delete** — single file (no directory recursion); not undoable from the app.
5. In edit mode secrets are **not** masked. Do not save a masked preview that still contains `***`.
6. After save the backend returns the previous text for one-level undo in the UI; no `.bak` is written. Undo does not cover rename/delete.
7. Settings diff across scopes remains **view-only**.

Untouched: `.credentials.json`, `~/.claude.json`, managed/system paths, files outside the allowlist.

Not in the product yet: creating artifacts from scratch, GUI settings forms, running hooks/MCP.

## Data locations

| What | Where |
|------|--------|
| App preferences / scan roots / change history | `~/Library/Application Support/com.agentscope.desktop/agent-scope.sqlite` |
| Claude Code configs | `~/.claude` (or `CLAUDE_CONFIG_DIR`), project `.claude/` |
| Cursor configs | `~/.cursor`, project `.cursor/` |

## Updates

The app can check [GitHub Releases](https://github.com/NejaTive/agent-scope/releases) for newer versions. Homebrew users can also `brew upgrade --cask agent-scope`.

## See also

- [Homebrew maintainer notes](homebrew.md)
- Config maps: [Claude Code](spec/claude-code.md), [Cursor](spec/cursor.md)
