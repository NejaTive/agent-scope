# Agent Scope

Локальное macOS-приложение для **просмотра и анализа** конфигурации Claude Code и Cursor — глобальной и проектной областей — без изменения файлов.

**v0.2** — поддержка Cursor (наряду с Claude Code).

Стек: **Tauri 2**, **Rust**, **Vue 3**, TypeScript, Tailwind.

## Что делает

- Сканирует `~/.claude` (или `CLAUDE_CONFIG_DIR`), `~/.cursor` и управляемые пути macOS
- Находит проекты в выбранных корнях (или добавляет проект вручную)
- Показывает Skills, Commands, Rules, Agents, Hooks, MCP, Plugins, Settings, Memory (`AGENTS.md`, `SKILL.md` и др.)
- Показывает эффективную конфигурацию с явной уверенностью (`computed` / `inferred` / `unknown`)
- Обнаруживает конфликты (дубли имён, расхождения MCP, переопределения настроек)
- Отслеживает файлы конфигурации и показывает недавние изменения

## Чего не делает

- Не создаёт, не редактирует и не удаляет конфигурацию Claude Code или Cursor (UI только для чтения)
- Не запускает хуки, MCP-серверы и shell-команды из конфигов
- По умолчанию не показывает содержимое credential-файлов и незамаскированные секреты

## Требования

- macOS 11+ (Big Sur), **только Apple Silicon (aarch64)** — Intel не поддерживается
- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) stable
- Xcode Command Line Tools

## Установка / Homebrew

Только Apple Silicon. Без нотаризации Gatekeeper может предупредить при первом запуске (Правый клик → Открыть, или снять quarantine через `xattr`).

```bash
brew tap nejative/agent-scope
brew install --cask agent-scope
# обновление
brew upgrade --cask agent-scope
```

Для мейнтейнеров: как обновлять tap и релизный чеклист — [docs/homebrew.md](docs/homebrew.md).

## Разработка

```bash
npm install
npm run tauri:dev
```

Только фронтенд (без IPC):

```bash
npm run dev
```

## Тесты

```bash
cd src-tauri && cargo test
npm run build   # vue-tsc + vite
```

## Сборка macOS-приложения / DMG

```bash
npm run tauri:build
```

Артефакты: `src-tauri/target/release/bundle/` (`.app`, `.dmg`).

### Замечания

- При выборе корней сканирования / проектов нужно дать доступ к папкам (минимальный TCC; Full Disk Access для выбранных пользователем путей не требуется).
- Первый скан больших деревьев может занять несколько секунд; исключаются: `node_modules`, `target`, `dist`, `build`, `.git` и т.п.

## Данные приложения

SQLite (предпочтения / корни сканирования / история изменений):

`~/Library/Application Support/com.agentscope.desktop/agent-scope.sqlite`

Конфиги Claude Code и Cursor остаются в исходных файлах.

## Спецификация

- [docs/spec/claude-code.md](docs/spec/claude-code.md) — карта конфигурации Claude Code
- [docs/spec/cursor.md](docs/spec/cursor.md) — карта конфигурации Cursor

## Лицензия

Private / pet project, если не указано иное.
