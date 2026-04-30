# Shell + Navigation → Tauri spec — 2026-04-29

> **Pre-cutover source note:** документ був написаний під час перенесення зі Slint. Посилання на `src/bootstrap/*` і `ui/*.slint` нижче є historical reference; live shell/navigation contract зараз у `frontend/src/App.svelte`, `frontend/src/lib/stores/navigation.ts`, `frontend/src/lib/stores/shell.ts` і `src-tauri/src/commands/shell.rs`.

## Призначення

Цей документ фіксує цільовий Tauri/frontend contract для root shell, navigation, company switcher і command palette.

Основні джерела:

- [src/bootstrap/navigation.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/navigation.rs:1)
- [src/bootstrap/shell.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/shell.rs:1)
- [src/bootstrap/palette.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:1)
- [src/bootstrap/company_switcher.rs](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/company_switcher.rs:1)
- [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:345)
- [ui/app.slint](/C:/Users/MykhailoDan/apps/Acta/ui/app.slint:40)

## Що є shell-domain

У межах міграції shell охоплює:

- root navigation по екранах;
- company switcher;
- top chrome;
- command palette;
- keyboard shortcuts;
- theme toggle;
- переходи в екрани через палітру.

## Поточні Slint callbacks

### Root app callbacks

Джерело:

- [ui/app.slint](/C:/Users/MykhailoDan/apps/Acta/ui/app.slint:40)

Важливі callbacks:

- `nav-changed(NavScreen)`
- `company-selected(string)`
- `palette-query-changed(string)`
- `palette-item-activated(string)`
- `settings-dark-mode-toggled(bool)`

### Shell callbacks

Джерело:

- [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:890)

Callbacks:

- `navigate(NavScreen)`
- `company-selected(string)`
- `company-manage-requested`
- `toggle-theme`
- `open-cmd-palette`
- `close-cmd-palette`
- `cmd-palette-query-changed(string)`
- `cmd-palette-item-activated(string)`

## Поточна Rust wiring логіка

### Navigation

Джерело:

- [wire_navigation](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/navigation.rs:35)

Що робить:

- мапить `NavScreen -> AppScreen`;
- оновлює `ctx.active_screen`;
- запускає refresh поточного екрана.

### Shell state

Джерело:

- [load_shell_state](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/shell.rs:61)
- [apply_shell_state](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/shell.rs:99)

Що робить:

- завантажує список компаній;
- формує `ShellChrome`;
- формує `CompanySwitcherItem[]`;
- пушить це в UI.

### Company switcher

Джерело:

- [wire_company_switcher](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/company_switcher.rs:12)

Що робить:

- змінює активну компанію;
- робить `refresh_all_ui`.

### Palette

Джерело:

- [wire_palette_callbacks](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:154)

Що робить:

- шукає palette items;
- активує navigate/open/create сценарії;
- може відкрити counterparty;
- може відкрити document;
- може стартувати create flow для документа.

## DTO

### `ShellChromeDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:211)

Поля:

- `companyName`
- `userName`
- `userInitials`
- `userRole`
- `documentsBadge`
- `tasksBadge`

### `CompanySwitcherItemDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:54)

Поля:

- `id`
- `name`
- `subtitle`
- `initials`
- `badge`
- `active`

### `PaletteItemDataDto`

Базується на:

- [ui/types.slint](/C:/Users/MykhailoDan/apps/Acta/ui/types.slint:46)

Поля:

- `kind`
- `title`
- `subtitle`
- `shortcut`
- `payload`

### `ShellStateDto`

Рекомендовані поля:

- `chrome: ShellChromeDto`
- `companyItems: CompanySwitcherItemDto[]`
- `isDark: bool`

## Команди

### `shell_load() -> ShellStateDto`

Джерело логіки:

- [load_shell_state](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/shell.rs:61)

Призначення:

- завантажити shell chrome;
- завантажити список компаній;
- повернути стартовий shell state.

### `shell_set_active_company(company_id) -> ShellStateDto`

Джерело логіки:

- [wire_company_switcher](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/company_switcher.rs:12)

Призначення:

- змінити активну компанію;
- повернути оновлений shell state;
- тригернути frontend invalidation усіх екранів.

### `shell_palette_search(query, selected_counterparty_id?) -> PaletteSearchResultDto`

Джерело логіки:

- [search_palette_items](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:165)

Призначення:

- повернути список palette items для поточного query.

### `shell_palette_activate(payload) -> PaletteActivationResultDto`

Джерело логіки:

- [parse_palette_action](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:207)
- [open_palette_counterparty](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:57)
- [open_palette_document](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:82)
- [start_palette_create_flow](/C:/Users/MykhailoDan/apps/Acta/src/bootstrap/palette.rs:97)

Призначення:

- інтерпретувати payload;
- сказати frontend, що треба зробити:
  - навігація;
  - відкриття counterparty;
  - відкриття document;
  - старт create flow.

Важливий архітектурний момент:

У Tauri цю команду краще робити не як side-effect-only, а як команду, що повертає структурований результат для frontend orchestration.

### `shell_toggle_theme(is_dark) -> ThemeMutationResultDto`

Поточний стан:

- theme toggle переважно живе в Slint:
  - [ui/app.slint](/C:/Users/MykhailoDan/apps/Acta/ui/app.slint:203)
  - [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:1439)

Ціль:

- або лишити це як frontend-only state;
- або зберігати theme preference через окремий backend config command.

Рекомендація:

Для першого етапу міграції лишити theme toggle у frontend.

## Що не повинно бути Tauri command

Ось що має лишитися у frontend:

- відкритість/закритість палітри;
- query у полі палітри;
- відкритість shortcuts help overlay;
- відкритість company switcher popup;
- локальний selected/highlighted state у списку palette items;
- keyboard event handling для `Ctrl+1..7`, `Ctrl+K`, `Ctrl+/`.

Це видно в Slint як локальний UI state:

- [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:855)
- [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:860)
- [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:1115)

## Keyboard shortcuts

Поточні shortcuts задокументовані в shell:

- `Ctrl+1..7` — навігація:
  - [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:739)
- `Ctrl+K` — command palette:
  - [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:781)
- `Ctrl+/` — overlay гарячих клавіш:
  - [ui/shell.slint](/C:/Users/MykhailoDan/apps/Acta/ui/shell.slint:782)

У Tauri/Svelte це має перейти в frontend keyboard handler, а не в Rust commands.

## Рекомендована frontend структура

- `src/lib/stores/navigation.ts`
- `src/lib/stores/shell.ts`
- `src/lib/stores/palette.ts`
- `src/lib/stores/theme.ts`
- `src/lib/components/Shell.svelte`
- `src/lib/components/Sidebar.svelte`
- `src/lib/components/CommandPalette.svelte`
- `src/lib/components/CompanySwitcher.svelte`

## Рекомендований мінімальний набір для першого vertical slice

1. `shell_load`
2. `shell_set_active_company`
3. `shell_palette_search`
4. `shell_palette_activate`

Все інше:

- navigation store
- palette open/close
- theme toggle
- shortcuts overlay

краще реалізувати локально у frontend.

## Основні ризики при переносі

1. Зараз shell navigation тісно зшитий із refresh flow через `AppScreen`.
2. Зараз company switcher робить `refresh_all_ui`, а не точкову invalidation.
3. Palette activation зараз виконує і navigation, і відкриття форм, і create orchestration в одному wiring модулі.
4. Theme toggle зараз частково живе в Slint, тому його легко втратити при cutover.

## Пов'язані документи

- [tauri-migration-contract-matrix-2026-04-29.md](./tauri-migration-contract-matrix-2026-04-29.md)
- [tauri-migration-roadmap-2026-04-29.md](./tauri-migration-roadmap-2026-04-29.md)
- [tauri-payments-command-spec-2026-04-29.md](./tauri-payments-command-spec-2026-04-29.md)
