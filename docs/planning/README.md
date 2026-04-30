# Planning

Оновлено: `2026-04-30`

Ця папка містить planning-нотатки й короткі звіти по спринтах у `Acta`.

## Live post-cutover documents

- [tauri-migration-audit-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-migration-audit-2026-04-29.md) — актуальний post-cutover аудит Tauri runtime
- [tauri-migration-roadmap-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-migration-roadmap-2026-04-29.md) — roadmap/backlog для Tauri після cutover
- [tauri-migration-contract-matrix-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-migration-contract-matrix-2026-04-29.md) — live Tauri invoke/frontend contract matrix
- [dashboard-migration-contract-2026-04-30.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/dashboard-migration-contract-2026-04-30.md) — рішення про redesign-first dashboard
- [tauri-documents-command-spec-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-documents-command-spec-2026-04-29.md) — documents command spec
- [tauri-counterparties-command-spec-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-counterparties-command-spec-2026-04-29.md) — counterparties command spec
- [tauri-payments-command-spec-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-payments-command-spec-2026-04-29.md) — payments command spec
- [tauri-shell-navigation-spec-2026-04-29.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/tauri-shell-navigation-spec-2026-04-29.md) — shell/navigation spec

## Archived/pre-cutover documents

- [next-sprint-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/next-sprint-2026-04-24.md) — детальний план спринту, сформований 2026-04-24
- [next-sprint-checklist-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/next-sprint-checklist-2026-04-24.md) — execution checklist для цього плану
- [sprint-report-2026-04-24.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/sprint-report-2026-04-24.md) — фактичний стан виконання плану, звірений з кодом 2026-04-27
- [remediation-sprint-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-sprint-2026-04-27.md) — актуальний remediation-план на найближчий тиждень
- [remediation-sprint-checklist-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-sprint-checklist-2026-04-27.md) — коротка execution-черга для remediation-плану
- [remediation-master-plan-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-master-plan-2026-04-27.md) — повний довгостроковий remediation-план з архітектурним порядком робіт
- [remediation-callback-matrix-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-callback-matrix-2026-04-27.md) — карта callback-ів, реального status flow і user-facing боргу
- [remediation-week-1-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-week-1-2026-04-27.md) — покроковий backlog на перший тиждень рефакторингу
- [remediation-week-2-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-week-2-2026-04-27.md) — покроковий backlog на другий тиждень, сфокусований на documents flow
- [document-chains-design-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/document-chains-design-2026-04-27.md) — design-нотатка для реалізації document chains
- [slint-final-cascade-removal-2026-04-30.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/slint-final-cascade-removal-2026-04-30.md) — archived execution note про видалення Slint cascade
- [slint-safe-removal-checklist-2026-04-30.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/slint-safe-removal-checklist-2026-04-30.md) — archived checklist для Slint cutover cleanup

## Контекст

Пакет зібрано на основі:

- відкритих пунктів у Vault `Acta`
- фактичних `TODO` / stub callback-ів у репозиторії
- поточного стану модулів у репозиторії
- повторної звірки planning-файлів з кодом станом на `2026-04-27`

## Правило використання

- `next-sprint-2026-04-24.md` — це план і цільовий scope, а не факт виконання
- `next-sprint-checklist-2026-04-24.md` — це робочий checklist цього плану, без претензії на статус-репорт
- `sprint-report-2026-04-24.md` — це єдиний файл у цій папці, який описує фактичний результат і розбіжності з планом
- `remediation-sprint-2026-04-27.md` — це поточний рекомендований план дій після звірки з кодом
- `remediation-sprint-checklist-2026-04-27.md` — це оперативна черга виконання цього remediation-плану
- `remediation-master-plan-2026-04-27.md` — це канонічний довгостроковий backlog remediation-робіт
- `remediation-callback-matrix-2026-04-27.md` — це швидка карта того, де callback already works, а де ще лишається борг
- `remediation-week-1-2026-04-27.md` та `remediation-week-2-2026-04-27.md` — це покроковий execution-план для джунів і нових учасників
- `document-chains-design-2026-04-27.md` — це технічна опора перед реалізацією chain flow, щоб не будувати його навмання
- Усі Slint/remediation документи до cutover читати як pre-cutover historical context.
- Нові UI/design-system рішення фіксувати в Svelte/Tauri docs, не в Slint token/component docs.
