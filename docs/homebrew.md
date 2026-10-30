# Homebrew (cask)

Пользовательский tap: `brew tap nejative/agent-scope` → репозиторий [NejaTive/homebrew-agent-scope](https://github.com/NejaTive/homebrew-agent-scope).

Cask token: `agent-scope`. Установка только на **Apple Silicon (aarch64)**; Intel не поддерживается.

Шаблон cask в этом репозитории: [`packaging/homebrew/Casks/agent-scope.rb`](../packaging/homebrew/Casks/agent-scope.rb).

## Создать / обновить tap

1. Создай (или клонируй) репозиторий `NejaTive/homebrew-agent-scope` на GitHub.
2. Скопируй шаблон из `packaging/homebrew/Casks/agent-scope.rb` этого репо в `Casks/agent-scope.rb` tap-репозитория.
3. Заполни `sha256` реальным хешем DMG из GitHub Release (см. чеклист ниже).
4. Закоммить и запушь в `homebrew-agent-scope`.

Структура tap:

```text
homebrew-agent-scope/
  Casks/
    agent-scope.rb
```

## Чеклист релиза (обновление tap)

После нового релиза приложения:

1. Создай тег `vX.Y.Z` в `agent-scope` и дождись GitHub Release с DMG.
2. Имя артефакта на GitHub Release (aarch64): `Agent.Scope_X.Y.Z_aarch64.dmg`
   (локально Tauri может собрать `Agent Scope_…` с пробелом; workflow публикует имя без пробелов — GitHub всё равно заменяет пробелы на точки).
3. Посчитай sha256 артефакта, например:
   ```bash
   shasum -a 256 "Agent.Scope_X.Y.Z_aarch64.dmg"
   ```
4. В `NejaTive/homebrew-agent-scope` обнови `Casks/agent-scope.rb`:
   - `version` → `X.Y.Z`
   - `sha256` → хеш из шага 3
5. Закоммить и запушь tap.
6. Проверь локально:
   ```bash
   brew update
   brew upgrade --cask agent-scope
   ```

URL в cask (форма):

`https://github.com/NejaTive/agent-scope/releases/download/v#{version}/Agent.Scope_#{version}_aarch64.dmg`

## Gatekeeper / нотаризация

Без подписи Developer ID и нотаризации macOS Gatekeeper может блокировать приложение при первом запуске («не удалось открыть, так как разработчик не может быть проверен»).

Обход для пользователя:

- Правый клик по приложению → **Открыть** → подтвердить, или
- снять quarantine: `xattr -dr com.apple.quarantine "/Applications/Agent Scope.app"`

Чтобы предупреждение исчезло «из коробки», нужна подпись + нотаризация в CI релиза (см. ниже).

## Секреты Apple (опционально, для CI)

Релизный workflow: [`.github/workflows/release.yml`](../.github/workflows/release.yml) (push тегов `v*`). Подпись/нотаризация включаются только если секреты заданы; иначе публикуется unsigned DMG. При настройке обычно нужны:

- `APPLE_CERTIFICATE` — base64 экспорт `.p12`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_ID`
- `APPLE_TEAM_ID`
- `APPLE_APP_SPECIFIC_PASSWORD` — или ключ App Store Connect API

Без этих секретов релизный DMG остаётся unsigned; Homebrew-установка всё равно работает, но Gatekeeper будет ругаться (см. выше).

## Архитектура

Cask рассчитан только на `arm64` / aarch64. На Intel `brew install --cask agent-scope` должен отказать — отдельного Intel URL нет.
