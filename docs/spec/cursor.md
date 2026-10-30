# Cursor configuration map (Agent Scope)

**Spec cut date:** 2026-10-31  
**Sources:** [cursor.com/docs/rules](https://cursor.com/docs/rules), [skills](https://cursor.com/docs/skills), [hooks](https://cursor.com/docs/hooks), [MCP](https://cursor.com/docs/context/mcp), [subagents](https://cursor.com/docs/subagents), [customize](https://cursor.com/docs/customize-cursor)

Agent Scope treats Cursor docs as source of truth. When formats drift, update this file and parsers together. Prefer docs over any internal orchestrator map; note divergences below when they appear.

## Config home

- User: `~/.cursor/`
- Project: `<root>/.cursor/`
- Managed / Enterprise hooks (macOS): `/Library/Application Support/Cursor/hooks.json`
- Linux/WSL managed hooks: `/etc/cursor/hooks.json`
- Windows managed hooks: `C:\ProgramData\Cursor\hooks.json`

## Rules & instructions

| Kind | Locations | Notes |
|------|-----------|-------|
| Project rules | `.cursor/rules/**/*.mdc` | **`.mdc` required.** Plain `.md` under `.cursor/rules` is ignored (no frontmatter for `description` / `globs` / `alwaysApply`). Nested folders allowed. |
| User rules | Customize → Rules (UI) | Global preferences for Agent (Chat). Docs state user rules are **not** stored as ordinary project files; on-disk path (if any) is **unknown** → confidence `unknown` when no file is found. |
| Team rules | Cursor dashboard (Team / Enterprise) | Free-form text; optional globs. Managed remotely; may be enforced so users cannot disable. |
| AGENTS.md | Project root + nested subdirectories | Plain markdown alternative to `.cursor/rules`. Nested files combine with parents; more specific wins. |

### Frontmatter (project `.mdc`)

| Field | Role |
|-------|------|
| `alwaysApply` | `true` → always included; globs/description ignored |
| `globs` | Path patterns; auto-attach when matching files are in context |
| `description` | When `alwaysApply` is false and globs omitted → Agent may apply by relevance |

Rule types in UI map to those fields: Always Apply / Apply Intelligently / Apply to Specific Files / Apply Manually (`@`-mention).

### Rules precedence

From docs (when guidance conflicts; all applicable rules still merge):

**Team → Project → User**

Team rules take precedence over project and user rules.

## Skills

Each skill is a folder containing `SKILL.md` (YAML frontmatter + body). Optional: `scripts/`, `references/`, `assets/`. Cursor walks skill roots recursively; nested category folders are organizational only — identity is the folder that holds `SKILL.md`. Nested project `.cursor/skills/` (or `.agents/skills/`) under monorepo packages are scoped to that subtree.

| Location | Scope |
|----------|-------|
| `.cursor/skills/` | Project |
| `.agents/skills/` | Project |
| `~/.cursor/skills/` | User (local; optional sync to Cloud Agents) |
| `~/.agents/skills/` | User (local; **not** synced to Cloud Agents) |

**Compatibility dirs** (also loaded): `.claude/skills/`, `.codex/skills/`, `~/.claude/skills/`, `~/.codex/skills/`.

### SKILL.md frontmatter (docs)

| Field | Required | Notes |
|-------|----------|-------|
| `name` | Yes | Lowercase, numbers, hyphens; must match parent folder |
| `description` | Yes | Relevance for auto-invocation |
| `paths` | No | Glob scope (prefer over legacy `globs`) |
| `disable-model-invocation` | No | `true` → slash-only (`/skill-name`) |
| `icon` / `color` | No | Custom Mode badge |
| `metadata` | No | Arbitrary key-value |

## Commands

Customize docs describe **Commands** as markdown files invoked with `/` in Agent chat. Skills docs treat slash commands as migratable to skills (`/migrate-to-skills`; migrated skills use `disable-model-invocation: true`).

| Location | Notes |
|----------|-------|
| `.cursor/commands/*.md` | Project / workspace commands if present |
| `~/.cursor/commands/*.md` | User-level commands if present |

Confidence: locations inferred from Customize + migration wording; treat as `inferred` until a dedicated commands reference pins paths.

## Agents / subagents

Markdown files with YAML frontmatter (`name`, `description`, optional `model`, `readonly`, `is_background`, …).

| Type | Locations |
|------|-----------|
| Project | `.cursor/agents/`, also `.claude/agents/`, `.codex/agents/` |
| User | `~/.cursor/agents/`, also `~/.claude/agents/`, `~/.codex/agents/` |

Project wins on name conflict. Among same-scope paths, `.cursor/` takes precedence over `.claude/` / `.codex/`.

## Hooks

Declared in `hooks.json` (`version` + `hooks` map). Scripts typically live beside config (e.g. `.cursor/hooks/`, `~/.cursor/hooks/`). Agent Scope **never executes** hooks.

| Scope | Path |
|-------|------|
| Enterprise (MDM) | macOS: `/Library/Application Support/Cursor/hooks.json`; Linux: `/etc/cursor/hooks.json`; Windows: `C:\ProgramData\Cursor\hooks.json` |
| Team | Cloud dashboard (Enterprise); synced; local managed under `~/.cursor/managed/...` per docs |
| Project | `<root>/.cursor/hooks.json` |
| User | `~/.cursor/hooks.json` |
| Plugins | After the above; command hooks only |

### Merge behavior (docs)

- **All matching hooks from every source run**
- Permission merge: **deny > ask > allow** (any source)
- `user_message` / `agent_message` concatenated
- Other fields (e.g. `followup_message`): last response wins; merge order for those fields follows priority **Enterprise → Team → Project → User** (then plugins)

Priority (highest → lowest): Enterprise → Team → Project → User → (plugin hooks after).

Cloud agents load project (+ Enterprise team/managed) hooks; **user** `~/.cursor/hooks.json` is **not** available in cloud agents.

## MCP

Top-level key: `mcpServers`.

| Scope | File |
|-------|------|
| Project | `<root>/.cursor/mcp.json` |
| Global / user | `~/.cursor/mcp.json` |

Transports (docs): **stdio**, **SSE**, **Streamable HTTP**. Auth via `env`, `headers`, OAuth / static `auth` (`CLIENT_ID`, `CLIENT_SECRET`, `scopes`). Interpolation: `${env:NAME}`, `${userHome}`, `${workspaceFolder}`, etc.

Agent Scope must **mask secrets** in `env`, `headers`, `auth`, and similar fields. Agent Scope **never starts** MCP servers.

Team / dashboard MCP and allowlists are cloud/UI — out of scope for local file viewers unless mirrored on disk.

## Plugins

Distributable bundles (rules, skills, subagents, commands, MCP, hooks) via Marketplace / team marketplace / Customize.

Agent Scope v0.2: only **on-disk, safely readable** plugin artifacts if present locally. Team/cloud UI install state is out of scope.

## Confidence model (Effective Configuration)

| Level | Meaning |
|-------|---------|
| `computed` | Documented merge applied to known on-disk keys/files |
| `inferred` | Likely behavior (e.g. commands paths, nested skill scoping) |
| `unknown` | No reliable derivation (User Rules in UI only, Team Rules/hooks dashboard, runtime session, OAuth tokens, shell env resolution) |

Agent Scope does not claim 100% parity with a live Cursor session.

## Agent Scope safety

- Read-only viewer: never creates, edits, or deletes Cursor (or Claude Code) configs
- Never executes hooks, MCP servers, skill scripts, or shell from configs
- Mask credentials and secrets in MCP and related JSON

## Notes vs orchestrator map (v0.2)

Orchestrator map aligned with docs for: `~/.cursor/` / `.cursor/` homes, rules `.mdc` + `AGENTS.md`, skills dirs including Claude/Codex compat, hooks paths + deny>ask>allow, MCP `mcp.json` locations, managed macOS hooks path.

Documented here from docs beyond the short map: rules precedence Team→Project→User; subagent Claude/Codex paths; hooks Enterprise→Team→Project→User priority; MCP transports and auth masking; User Rules often UI-only (`unknown` if no file).
