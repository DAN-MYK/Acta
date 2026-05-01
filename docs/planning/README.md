# Planning

Оновлено: `2026-05-01`

Ця папка містить planning-нотатки та технічні специфікації для `Acta`.

## Активні плани

- [ui-ux-roadmap-2026-05-01.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/ui-ux-roadmap-2026-05-01.md) — живий backlog для post-cutover UI/UX polish.

## Канонічні архітектурні документи

Після завершення Tauri cutover корисний зміст migration/spec planning-нотаток перенесено в канонічні architecture docs:

- [tauri-runtime.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/tauri-runtime.md) — live runtime, cutover decisions, dashboard contract, archived Slint policy, CI contract.
- [tauri-command-surface.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/tauri-command-surface.md) — public invoke surface і frontend/backend contract rules.
- [ui-canonicalization.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/ui-canonicalization.md) — канонічний UI шлях після cutover.
- [app-state.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/app-state.md) — state/store model після cutover.
- [svelte-tauri-design-system.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/svelte-tauri-design-system.md) — design-system foundation для live runtime.

## Історичний контекст

Файли `next-sprint-*`, `remediation-*`, `slint-*`, `document-chains-design-*` у цій папці не є активним backlog. Їх потрібно читати лише як історичний або pre-cutover контекст, навіть якщо всередині залишилися статуси `open`, `planned` або незакриті checklist-пункти.

## Правило використання

- Якщо потрібен поточний план робіт, починай з документів у розділі `Активні плани`.
- Якщо потрібен фактичний post-cutover контракт, пріоритет мають документи в `docs/architecture/`, а не завершені planning-файли.
- Якщо в історичному файлі є відкриті пункти, не трактуй їх як поточні задачі без окремої звірки з кодом і актуальними Tauri/Svelte документами.
- Нові planning-нотатки для живого UI та архітектури потрібно фіксувати у Tauri/Svelte контексті, а не в Slint-era документах.
