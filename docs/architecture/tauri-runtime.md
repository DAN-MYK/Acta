# Tauri Runtime

Оновлено: `2026-05-01`

## Призначення

Цей документ є канонічним описом live desktop runtime для `Acta` після cutover на Tauri/Svelte.

## Канонічний runtime

Поточний runnable UI складається з:

- [src-tauri](/C:/Users/MykhailoDan/apps/Acta/src-tauri) — Tauri entrypoint, invoke handler і command wrappers;
- [frontend/src](/C:/Users/MykhailoDan/apps/Acta/frontend/src) — Svelte screens, stores, typed frontend API;
- [src/tauri_api](/C:/Users/MykhailoDan/apps/Acta/src/tauri_api) — backend DTO/command surface для Tauri;
- [src](/C:/Users/MykhailoDan/apps/Acta/src) — домен, DB, імпорт, PDF і shared backend logic.

## Що вважається завершеним

Для цілей cutover завершеними вважаються:

- Tauri scaffold;
- shared backend bootstrap через `AppCtx`;
- public command contract для live frontend surface;
- shell + feature screens у Svelte;
- refresh/store wiring model;
- design-system foundation для live runtime;
- Tauri/Frontend test і CI gates;
- фінальний cutover з видаленням live Slint runtime.

## Shell і feature screens

Етап `Shell + feature screens` вважається завершеним:

- root shell живе в [frontend/src/App.svelte](/C:/Users/MykhailoDan/apps/Acta/frontend/src/App.svelte);
- live screens існують для `dashboard`, `documents`, `counterparties`, `payments`, `reports`, `tasks`, `settings`;
- для цих slices є відповідні stores у [frontend/src/lib/stores](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/stores);
- public Tauri commands для shell і feature screens зареєстровані в [src-tauri/src/lib.rs](/C:/Users/MykhailoDan/apps/Acta/src-tauri/src/lib.rs).

Незакриті UX-питання по цих екранах належать до post-cutover product/UI backlog, а не до незавершеного migration stage.

## Design-system foundation

Етап `Дизайн-система` для міграції завершено на рівні foundation:

- канонічні tokens живуть у [frontend/src/lib/styles/tokens.css](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/styles/tokens.css);
- глобальні layout/style rules живуть у [frontend/src/styles.css](/C:/Users/MykhailoDan/apps/Acta/frontend/src/styles.css);
- design-system правила для live runtime зафіксовано в [svelte-tauri-design-system.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/svelte-tauri-design-system.md).

Подальша уніфікація кнопок, action bars, empty states, form controls і screen-level UX належить до окремого UI/UX roadmap.

## Dashboard contract

Dashboard у Tauri є `redesign-first` screen, а не strict parity-копією Slint dashboard.

Вважаємо реалізованим у поточному runtime:

- backend-backed завантаження dashboard;
- KPI-блок;
- cashflow summary;
- recent acts;
- upcoming payments;
- urgent tasks;
- переходи з dashboard у documents / payments / tasks.

Вважаємо свідомо виключеним із поточного contract:

- `journal`;
- `inbox`;
- accounts block;
- chart-first Slint layout;
- dashboard-level task actions;
- YTD/delta/sparkline presentation details.

Якщо ці сценарії знадобляться знову, це нові Tauri feature requirements, а не борг незавершеної parity-міграції.

## Archived Slint policy

Slint runtime більше не є live source of truth. Як historical reference дозволено дивитись лише archived/worktree контекст:

- `.worktrees/sprint-2026-04-24/ui/*.slint`;
- `.worktrees/sprint-2026-04-24/src/ui/*`;
- старі planning/audit docs, якщо вони явно трактуються як archived або pre-cutover reference.

Не використовувати як live source:

- `ui/`;
- root `build.rs`;
- `src/ui/*`;
- `src/bootstrap/*` Slint wiring;
- `tests/ui_events.rs`.

## CI contract

Мінімальний post-cutover CI має покривати:

- frontend typecheck/build/tests: `npm run check`, `npm run build`, `npm run test:frontend`;
- Rust backend compile/tests;
- DB-backed vertical slice: `cargo test --test tauri_vertical_slice`;
- Linux Tauri compile smoke;
- Linux real WebView e2e smoke;
- Windows Tauri packaging gate.

Windows packaging gate є частиною поточного release contract, а не майбутнім TODO.

## Пов’язані документи

- [ui-canonicalization.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/ui-canonicalization.md)
- [app-state.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/app-state.md)
- [svelte-tauri-design-system.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/svelte-tauri-design-system.md)
