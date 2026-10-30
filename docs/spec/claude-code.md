# Claude Code configuration map (Agent Scope)

**Spec cut date:** 2026-10-30  
**Sources:** [code.claude.com/docs](https://code.claude.com/docs/en/claude-directory)

Agent Scope treats Anthropic docs as source of truth. When formats drift, update this file and parsers together.

## Config home

- Default: `~/.claude`
- Override: `CLAUDE_CONFIG_DIR`
- Global app state / user+local MCP: `~/.claude.json` (or beside `CLAUDE_CONFIG_DIR`)

## Settings files & precedence

| Scope | File |
|-------|------|
| Managed | `/Library/Application Support/ClaudeCode/managed-settings.json` (macOS) |
| Project local | `.claude/settings.local.json` |
| Project | `.claude/settings.json` |
| User | `~/.claude/settings.json` |

Precedence (highest first): Managed → `--settings` (session, not modeled) → local → project → user.

List keys such as `permissions.allow` merge across scopes. Some model list keys have special rules — see settings reference.

**Note:** There is no official `~/.claude/settings.local.json`. Local is project-only.

## Memory / instructions

- Managed: `/Library/Application Support/ClaudeCode/CLAUDE.md`
- User: `~/.claude/CLAUDE.md`
- Project: `CLAUDE.md` or `.claude/CLAUDE.md`
- Local: `CLAUDE.local.md`
- Optional: `AGENTS.md`
- Rules: `rules/**/*.md` with optional frontmatter `paths`
- Auto memory: `~/.claude/projects/<id>/memory/`

## Skills & commands

- Skills: `skills/<name>/SKILL.md` (+ supporting files)
- Commands (legacy): `commands/*.md` — same mechanism as skills
- Nested: `<subdir>/.claude/skills/...`

## Agents

- `agents/*.md` — YAML frontmatter; `name` + `description` required

## Hooks

- Declared under `hooks` in settings JSON (user / project / local / managed / plugins / skill|agent frontmatter)
- Agent Scope **never executes** hooks

## MCP

| Scope | Storage |
|-------|---------|
| Project | `<root>/.mcp.json` (`mcpServers`) — **not** under `.claude/` |
| User | `~/.claude.json` → `mcpServers` |
| Local | `~/.claude.json` → `projects[<path>].mcpServers` |

`settings.json` does **not** read a top-level `mcpServers` key.

## Plugins

- `~/.claude/plugins/` (marketplaces, installed record, synced)

## Confidence model (Effective Configuration)

| Level | Meaning |
|-------|---------|
| `computed` | Documented merge applied to known keys |
| `inferred` | Likely behavior (e.g. path-scoped rules) |
| `unknown` | No reliable derivation (runtime session, approvals, shell env pairs) |

Agent Scope does not claim 100% parity with a live Claude Code session.
