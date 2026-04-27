# Remediation Master Plan

Оновлено: `2026-04-27`  
Горизонт: `4-6 тижнів`  
Статус: `active`

## Мета

Цей документ описує не локальний sprint-fix, а повний план зменшення технічного боргу в `Acta` через великий, але контрольований рефакторинг.

Кінцева ціль:

- прибрати user-facing `TODO` із критичних сценаріїв;
- винести orchestration з перевантажених файлів;
- зробити flows однаково організованими між documents, counterparties, payments, tasks і settings;
- довести `migrate` до реального import pipeline;
- підготувати основу для подальшого розвитку без хаотичного росту callback-ів.

## Основні проблеми

### P1. User-facing борг

- documents screen має дії, що ведуть лише в `TODO`;
- counterparties screen має незавершені create-flow сценарії;
- `migrate` ще не є реальним BAS import flow;
- document chains wired частково, але не завершені як фіча.

### P2. Архітектурний борг

- `src/bootstrap.rs` містить занадто багато feature orchestration;
- частина бізнес-сценаріїв розмазана між `bootstrap.rs` та `src/ui/*.rs`;
- рядкові префікси документів (`act:`, `inv:`, `wbl:`) використовуються як імпровізований internal API;
- refresh/error handling pattern не достатньо уніфіковані.

### P3. Operational / support борг

- частина екранів тихо ковтає помилки через `unwrap_or_default()`;
- backup flow деградує до JSON snapshot без чіткої класифікації в UI;
- shell/UI polish ще має кілька тимчасових підстановок.

## Цільова архітектура

Після remediation бажана така структура:

- `src/main.rs` — тільки bootstrap entrypoint;
- `src/bootstrap.rs` — runtime, DB init, initial load, high-level wiring entrypoints;
- `src/actions/*` — application scenarios, команди, routing, refresh orchestration;
- `src/ui/*` — presenter layer: `prepare_*`, `apply_*`, thin callback adapters;
- `src/db/*` — SQL/repository layer;
- `ui/*.slint` — UI contract та presentation state;
- `src/import/bas/*` — окремий pipeline для імпорту.

## Робочі потоки

### Потік читання

`callback -> action -> prepare_* -> stale check -> apply_*_to_ui`

### Потік команди

`callback -> action -> DB mutation / service -> refresh affected screens -> optional notify/log`

## Основні workstreams

### 1. Application Layer Extraction

Створити новий шар:

- `src/actions/common.rs`
- `src/actions/navigation.rs`
- `src/actions/search.rs`
- `src/actions/documents.rs`
- `src/actions/counterparties.rs`
- `src/actions/payments.rs`
- `src/actions/tasks.rs`
- `src/actions/settings.rs`

Ціль:

- перестати складати feature orchestration у `bootstrap.rs`;
- дати джунам зрозумілі точки входу для конкретних сценаріїв.

### 2. Documents Flow Completion

Побудувати повноцінний flow для:

- `doc_new`
- `doc_open`
- `doc_edit`
- bulk-операцій
- chain load / chain create

Не латати точково, а ввести:

- нормалізований `DocumentRef`;
- editor state;
- command handlers;
- спільний contract для create/edit/view.

### 3. Counterparties Flow Completion

Добудувати:

- створення контрагента;
- редагування;
- створення документа з контрагента;
- інтеграцію з documents editor.

### 4. BAS Import Pipeline

Перетворити `src/bin/migrate.rs` на thin CLI і винести імпорт у:

- scan
- parse
- map
- persist
- report

### 5. Refresh / Error Handling Cleanup

Уніфікувати:

- stale-check pattern;
- multi-screen refresh;
- error logging;
- свідомі fallback-и замість мовчазного ковтання помилок.

### 6. Settings / Backup Clarification

Чітко розділити:

- full DB backup;
- partial metadata snapshot;
- UI labeling backup type.

## Порядок виконання

1. Підготовка документації та callback matrix.
2. `actions/` skeleton.
3. Винесення navigation і palette/search.
4. Винесення documents command flows.
5. Documents editor + create/edit/open.
6. Bulk actions + chains.
7. Counterparties editor + create-doc bridge.
8. BAS import pipeline.
9. Error handling / backup cleanup.
10. Final debt sweep.

## Контрольні артефакти

Цей master-plan працює разом із такими файлами:

- [remediation-callback-matrix-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-callback-matrix-2026-04-27.md)
- [remediation-week-1-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-week-1-2026-04-27.md)
- [remediation-week-2-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/remediation-week-2-2026-04-27.md)
- [document-chains-design-2026-04-27.md](/C:/Users/MykhailoDan/apps/Acta/docs/planning/document-chains-design-2026-04-27.md)

## Definition of Done

Remediation вважається завершеним, коли одночасно виконуються всі умови:

- у `documents` і `counterparties` немає user-facing дій, що лише логують `TODO`;
- `migrate` виконує реальний dry-run або import flow;
- `bootstrap.rs` більше не є головним місцем feature orchestration;
- основні screen loaders не ковтають критичні помилки мовчки;
- regression tests покривають раніше проблемні flows.
