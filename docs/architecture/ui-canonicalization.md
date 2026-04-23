# UI Canonicalization

Оновлено: `2026-04-23`

## Канонічний статус

Єдиний канонічний UI Acta — `ui/`.

- `build.rs` компілює тільки `ui/app.slint`.
- `slint::include_modules!()` у runtime працює з generated contract від `ui`.
- `tests/ui_events.rs` перевіряє актуальний `AppWindow` із `ui`.
- Папка `ui/` не є runtime-джерелом істини.

## Inventory legacy UI artifacts

| Артефакт | Статус | Рішення |
|---|---|---|
| legacy `.slint` дерево в `ui/` | retired | `remove` |
| legacy Rust presenter-модулі `src/ui/{acts,companies,invoices,waybills}.rs` | retired | `remove` |
| runtime build path | уже на `ui/app.slint` | `keep` |
| `tests/ui_events.rs` | уже на новому contract | `keep` |
| посилання в інструкціях/документації на `ui/` як current UI | retired | `remove` |

## Правило для розробки

Усі нові UI-зміни, callback-и, screen contracts і accessibility-оновлення потрібно вносити тільки в `ui/`.

Legacy Slint-дерево в `ui/` та legacy Rust presenter-шар для старого `MainWindow`
в `src/ui/{acts,companies,invoices,waybills}.rs` уже прибрано з репозиторію.
Історичний контекст по міграції збережено лише в документації.

## Що вважається закритим у межах Epic 1

1. `ui` зафіксовано як єдиний current UI у build/runtime/docs.
2. Старий `MainWindow` не використовується як поточний binding contract.
3. Legacy UI не подає хибний сигнал як active runtime layer.
4. Є явний inventory legacy artifacts з рішенням `keep` або `remove`.
