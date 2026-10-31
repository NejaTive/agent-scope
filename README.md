# Agent Scope

Локальное macOS-приложение для **просмотра и анализа** конфигурации Claude Code и Cursor — глобальной и проектной областей. Запись существующих файлов из allowlist — **opt-in**.

**v0.3** — безопасное редактирование allowlisted конфигов (по умолчанию выключено).

Стек: **Tauri 2**, **Rust**, **Vue 3**, TypeScript, Tailwind.

## Что делает

- Сканирует `~/.claude` (или `CLAUDE_CONFIG_DIR`), `~/.cursor` и управляемые пути macOS
- Находит проекты в выбранных корнях (или добавляет проект вручную)
- Показывает Skills, Commands, Rules, Agents, Hooks, MCP, Plugins, Settings, Memory (`AGENTS.md`, `SKILL.md` и др.)
- Показывает эффективную конфигурацию с явной уверенностью (`computed` / `inferred` / `unknown`)
- Обнаруживает конфликты (дубли имён, расхождения MCP, переопределения настроек)
- Отслеживает файлы конфигурации и показывает недавние изменения
- По желанию правит / переименовывает / удаляет **существующие** файлы из allowlist (markdown rules/skills/commands/agents/memory, JSON settings/MCP/hooks) — см. ниже

## Редактирование (opt-in)

1. В настройках приложения включите «Разрешить редактирование файлов конфигурации» (`allowEditing`, по умолчанию **выкл.**).
2. Откройте артефакт → **Изменить** → правьте сырой файл → **Сохранить**; либо **Переименовать** / **Удалить**.
3. Rename — только новое имя в **той же папке**; dest не должен существовать и должен оставаться в allowlist (например `CLAUDE.md` → `FOO.md` в корне проекта отклоняется).
4. Delete — один файл (без рекурсии каталогов); из приложения не откатывается.
5. В режиме правки секреты **не маскируются**. Не сохраняйте замаскированный просмотр с `***`.
6. После save бэкенд возвращает предыдущий текст для одноуровневого undo в UI; `.bak` на диск **не** пишется. Undo для rename/delete нет.
7. Effective Config / diff / resolved settings — **только просмотр**.

Не трогаются: `.credentials.json`, `~/.claude.json`, managed/system пути, файлы вне allowlist. Не в MVP: создание артефактов с нуля, GUI-формы settings, merge effective config, запуск hooks/MCP.

## Чего не делает

- Не создаёт новые артефакты с нуля; не пишет credentials и пути вне allowlist
- Не запускает хуки, MCP-серверы и shell-команды из конфигов
- По умолчанию (без Edit mode) не показывает незамаскированные секреты; credential-файлы не открывает

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
