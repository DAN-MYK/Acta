# Planning

Оновлено: `2026-05-01`

Ця папка містить planning-нотатки та технічні специфікації для `Acta`.

## Активні плани

- [tauri-migration-audit-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-migration-audit-2026-04-29.md) — актуальний post-cutover аудит Tauri runtime.
- [tauri-migration-roadmap-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-migration-roadmap-2026-04-29.md) — поточний roadmap і backlog після cutover.
- [tauri-migration-contract-matrix-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-migration-contract-matrix-2026-04-29.md) — жива матриця frontend/invoke контрактів.
- [dashboard-migration-contract-2026-04-30.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/dashboard-migration-contract-2026-04-30.md) — актуальний контракт redesign-first dashboard.
- [tauri-documents-command-spec-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-documents-command-spec-2026-04-29.md) — актуальна специфікація команд для documents.
- [tauri-counterparties-command-spec-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-counterparties-command-spec-2026-04-29.md) — актуальна специфікація команд для counterparties.
- [tauri-payments-command-spec-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-payments-command-spec-2026-04-29.md) — актуальна специфікація команд для payments.
- [tauri-shell-navigation-spec-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-shell-navigation-spec-2026-04-29.md) — актуальна специфікація shell і navigation.

## Історичний контекст

Файли `next-sprint-*`, `remediation-*`, `slint-*`, `document-chains-design-*` у цій папці не є активним backlog. Їх потрібно читати лише як історичний або pre-cutover контекст, навіть якщо всередині залишилися статуси `open`, `planned` або незакриті checklist-пункти.

## Правило використання

- Якщо потрібен поточний план робіт, починай з документів у розділі `Активні плани`.
- Якщо потрібен фактичний post-cutover контракт, пріоритет мають `tauri-*-spec`, `tauri-migration-*` і `dashboard-migration-contract-*`.
- Якщо в історичному файлі є відкриті пункти, не трактуй їх як поточні задачі без окремої звірки з кодом і актуальними Tauri/Svelte документами.
- Нові planning-нотатки для живого UI та архітектури потрібно фіксувати у Tauri/Svelte контексті, а не в Slint-era документах.
