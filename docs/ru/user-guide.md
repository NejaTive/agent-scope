# Руководство пользователя

> English: [../user-guide.md](../user-guide.md)

Agent Scope — локальное macOS-приложение для **просмотра и (по желанию) правки** конфигурации Claude Code и Cursor — глобальной и проектной областей.

## Установка

Только **Apple Silicon** (Intel не поддерживается). macOS 11+.

```bash
brew tap nejative/agent-scope
brew install --cask agent-scope
# обновление
brew upgrade --cask agent-scope
```

Или скачайте `.dmg` с [GitHub Releases](https://github.com/NejaTive/agent-scope/releases).

### Gatekeeper

Unsigned-сборка может блокироваться при первом запуске. Правый клик → **Открыть**, или снять quarantine:

```bash
xattr -dr com.apple.quarantine "/Applications/Agent Scope.app"
```

## Первый запуск

1. Дайте доступ к папкам при выборе корней сканирования / проектов (минимальный TCC; Full Disk Access для выбранных вами путей не нужен).
2. Дождитесь первого скана — большие деревья могут занять несколько секунд. Исключаются: `node_modules`, `target`, `dist`, `build`, `.git` и т.п.
3. Сайдбар: Обзор, Глобальные, Проекты, Скиллы, Правила, Агенты, Хуки, MCP, Плагины, Права, Настройки, Память.

## Что показывает

| Область | Примеры |
|---------|---------|
| Skills и команды | `SKILL.md`, slash-команды |
| Правила и агенты | user/project rules, определения агентов |
| Хуки и MCP | конфиги hooks, записи MCP-серверов |
| Плагины и права | списки плагинов, permissions |
| Настройки | scoped settings + diff (только просмотр) |
| Память | `AGENTS.md`, `CLAUDE.md`, memory-файлы |

На обзоре видны конфликты: дубли имён, расхождения MCP, переопределения настроек. После сканов отслеживаются недавние изменения файлов.

## Редактирование (opt-in)

По умолчанию **выключено**.

1. Настройки приложения → включите **Разрешить редактирование файлов конфигурации** (`allowEditing`).
2. Артефакт → **Изменить** → правка сырого файла → **Сохранить**; либо **Переименовать** / **Удалить**.
3. **Rename** — только новое имя в **той же папке**; dest не должен существовать и должен оставаться в allowlist (например `CLAUDE.md` → `FOO.md` в корне проекта отклоняется).
4. **Delete** — один файл (без рекурсии каталогов); из приложения не откатывается.
5. В режиме правки секреты **не маскируются**. Не сохраняйте замаскированный просмотр с `***`.
6. После save бэкенд возвращает предыдущий текст для одноуровневого undo в UI; `.bak` на диск **не** пишется. Undo для rename/delete нет.
7. Diff настроек по областям — **только просмотр**.

Не трогаются: `.credentials.json`, `~/.claude.json`, managed/system пути, файлы вне allowlist.

Пока нет: создание артефактов с нуля, GUI-формы settings, запуск hooks/MCP.

## Где лежат данные

| Что | Где |
|-----|-----|
| Предпочтения / корни скана / история изменений | `~/Library/Application Support/com.agentscope.desktop/agent-scope.sqlite` |
| Конфиги Claude Code | `~/.claude` (или `CLAUDE_CONFIG_DIR`), проектный `.claude/` |
| Конфиги Cursor | `~/.cursor`, проектный `.cursor/` |

## Обновления

Приложение может проверять [GitHub Releases](https://github.com/NejaTive/agent-scope/releases). Через Homebrew: `brew upgrade --cask agent-scope`.

## См. также

- [Homebrew для мейнтейнеров](../homebrew.md)
- Карты конфигов: [Claude Code](../spec/claude-code.md), [Cursor](../spec/cursor.md)
