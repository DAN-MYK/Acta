# UI Safety Net

Оновлено: `2026-04-23`

## Мета

`tests/ui_events.rs` — це headless safety net для канонічного Slint UI в `ui-redesign/`.

Тестовий файл призначений для швидкого виявлення regression по:

- root callback wiring;
- shell callback contracts;
- command palette contracts;
- базових navigation / inbox / documents / tasks / settings flows.

## Поточна структура

Файл `tests/ui_events.rs` поділено на логічні блоки:

- `helpers` — спільні helper-и для створення компонентів і capture state;
- `app_window_contract` — перевірки callback-ів `AppWindow`;
- `shell_contract` — перевірки shell-level callback contract;
- `keyboard_palette_regressions` — окремі regression-тести для shell/palette routing contract;
- `shell_test_components` — test-only host components для `Shell` і `CommandPalette`.

## Правила розширення

Коли додаєш новий interaction test:

1. Якщо callback належить `AppWindow`, тест додається в `app_window_contract`.
2. Якщо callback живе в `ui-redesign/shell.slint`, тест додається в `shell_contract` або `keyboard_palette_regressions`.
3. Якщо потрібен новий test-only wrapper для внутрішнього Slint component, додавай його в `shell_test_components` або окремий test-only host module.
4. Не повертайся до legacy `ui/` і не тестуй старий `MainWindow`.

## Чому тут string-based routing для shell host

У test-only wrappers для `Shell` і `CommandPalette` routing переведений у рядкові ідентифікатори (`"dashboard"`, `"documents"` тощо), бо окремі `slint!` host-компоненти генерують власні enum namespace-и.

Це зроблено навмисно, щоб regression-тести перевіряли стабільність callback contract, а не ламались через конфлікти generated типів.

## Мінімальний набір перевірок

Для `Epic 2` покрито:

- `nav-changed`
- `inbox-action`
- `doc-*`
- `task-*`
- `settings-*`
- `palette-*`
- `Shell.navigate`
- `Shell.toggle-theme`
- `Shell.open-cmd-palette`
- `Shell.close-cmd-palette`
- `CommandPalette.closed`
- `CommandPalette.navigated`
- `CommandPalette.query-changed`

## Базові команди перевірки

```bash
SQLX_OFFLINE=true cargo test --test ui_events --no-run
SQLX_OFFLINE=true cargo test --test ui_events
```
