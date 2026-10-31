# Agent Scope

<p align="center">
  <img src="../assets/app-icon.png" width="96" height="96" alt="Agent Scope">
</p>

<p align="center">
  <strong>Все конфиги Claude Code и Cursor — в одном окне.</strong><br>
  Локальное macOS-приложение: скан, конфликты, опциональное редактирование allowlist.
</p>

<p align="center">
  <a href="#установка">Установка</a> ·
  <a href="user-guide.md">Руководство</a> ·
  <a href="../../README.md">English</a> ·
  <a href="https://github.com/NejaTive/agent-scope/releases">Releases</a>
</p>

<!-- Скриншоты: положите PNG в docs/assets/ (см. docs/assets/README.md), затем раскомментируйте:
<p align="center">
  <img src="../assets/dashboard.png" width="800" alt="Обзор">
</p>
-->

## Зачем

Claude Code и Cursor размазывают skills, rules, agents, hooks, MCP и memory по глобальным и проектным деревьям. Agent Scope даёт один локальный UI, чтобы:

- **Инвентаризировать** `~/.claude` / `CLAUDE_CONFIG_DIR`, `~/.cursor` и выбранные проекты
- **Ловить конфликты** — дубли имён, расхождения MCP, переопределения настроек
- **Видеть изменения** после каждого скана
- **Безопасно править (opt-in)** — rename / delete / save только существующих файлов из allowlist; credentials и managed-пути не трогаются

Стек: **Tauri 2**, **Rust**, **Vue 3**, TypeScript, Tailwind.  
Актуальная версия: **v0.3** — безопасное редактирование allowlisted конфигов (по умолчанию выкл.).

## Установка

Только **Apple Silicon** · macOS 11 (Big Sur)+

```bash
brew tap nejative/agent-scope
brew install --cask agent-scope
```

Обновление: `brew upgrade --cask agent-scope`

DMG: [Releases](https://github.com/NejaTive/agent-scope/releases). При предупреждении Gatekeeper — правый клик → **Открыть** ([подробнее](user-guide.md#gatekeeper)).

## Возможности

| | |
|--|--|
| **Обзор** | Счётчики проектов, skills, rules, agents, hooks, MCP, plugins, memory |
| **Глобальный + проектный scope** | User-level и per-repo артефакты рядом |
| **Браузер артефактов** | Markdown / JSON + поиск |
| **Конфликты** | Дубли имён, MCP drift, переопределения настроек |
| **Лента изменений** | Недавние create / modify / delete |
| **Opt-in editing** | Сырая правка, rename в той же папке, удаление файла — выкл. по умолчанию |
| **Маскирование секретов** | В просмотре; без маски только в режиме правки |
| **Проверка обновлений** | Уведомление о новом GitHub Release |

**Не делает:** создание артефактов с нуля, запуск hooks/MCP/shell из конфигов, запись credentials и путей вне allowlist, поддержку Intel Mac.

## Быстрый старт из исходников

```bash
npm install
npm run tauri:dev
```

Тесты: `cd src-tauri && cargo test` · сборка фронта: `npm run build`  
DMG: `npm run tauri:build` → `src-tauri/target/release/bundle/`

Установка, правила правки, пути данных: **[user-guide.md](user-guide.md)**.

## Документация

| Документ | |
|----------|--|
| [Руководство пользователя](user-guide.md) | Установка, скан, редактирование |
| [Homebrew](../homebrew.md) | Чеклист релиза для мейнтейнеров |
| [Карта Claude Code](../spec/claude-code.md) · [Cursor](../spec/cursor.md) | Спеки раскладки |
| [English README](../../README.md) | Основная страница репозитория |

## Лицензия

[MIT](../../LICENSE) © Alexandr Sharov
