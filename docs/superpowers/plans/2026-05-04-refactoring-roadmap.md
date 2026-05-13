# Refactoring Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Перевести Acta з переважно layer-first структури на більш виражену feature-first організацію без зламу Tauri/Svelte контрактів і без великого одноразового переписування.

**Architecture:** Поточний код вже фактично організований по бізнес-slice'ах, але фізично розкладений по технічних шарах: `db`, `tauri_api`, `commands`, `stores`, `screens`. Рефакторинг має йти хвилями: спочатку зафіксувати цільову карту модулів і правила меж, потім переносити найменш зв'язані slice'и, і лише після цього чіпати важкі вузли на кшталт `documents`, `payments`, `import`, `pdf`.

**Tech Stack:** Rust, Tauri 2, Svelte 4, TypeScript, PostgreSQL, sqlx, Vitest, Rust integration tests

---

## Поточна карта системи

### Backend core

- `src/app_ctx.rs` — shared backend context (`PgPool` + `active_company_id`)
- `src/runtime.rs` — ініціалізація runtime та background tasks
- `src/db/*.rs` — query/repository шар по доменах
- `src/models/*.rs` — доменні моделі та frontend-facing DTO
- `src/tauri_api/*.rs` — application/use-case шар для Tauri commands
- `src/services/*.rs` — прикладні сервіси
- `src/import/*.rs` — імпорт BAS / bank
- `src/pdf/*.rs` — PDF генерація та читання
- `src/actions/*.rs` — shell/palette/inbox поведінка

### Adapter layer

- `src-tauri/src/lib.rs` — реєстрація invoke surface
- `src-tauri/src/commands/*.rs` — thin Tauri wrappers

### Frontend

- `frontend/src/App.svelte` — shell, навігація, palette
- `frontend/src/lib/api.ts` — invoke bridge
- `frontend/src/lib/types.ts` — TS contract
- `frontend/src/lib/stores/*.ts` — state по slice'ах
- `frontend/src/lib/screens/*.svelte` — screen-компоненти
- `frontend/src/lib/components/*.svelte` — shared UI components

## Цільова структура

### Backend target

```text
src/
├── app/
│   ├── shell/
│   ├── dashboard/
│   ├── counterparties/
│   ├── documents/
│   ├── payments/
│   ├── reports/
│   ├── tasks/
│   └── settings/
├── integrations/
│   ├── bas/
│   └── bank/
├── documents_io/
│   ├── pdf/
│   └── templates/
├── shared/
│   ├── app_ctx.rs
│   ├── config.rs
│   ├── runtime.rs
│   └── money.rs
└── lib.rs
```

### Frontend target

```text
frontend/src/lib/
├── shell/
├── dashboard/
├── counterparties/
├── documents/
├── payments/
├── reports/
├── tasks/
├── settings/
├── shared/
│   ├── api/
│   ├── components/
│   ├── styles/
│   └── types/
└── App.svelte
```

## Принципи рефакторингу

- Не міняти public Tauri command names без окремої причини.
- Не ламати money contract: суми в Rust як `Decimal`, у frontend-facing DTO як `string`.
- Не переносити кілька важких slice'ів одночасно.
- Кожен етап має завершуватися compile/test checkpoint.
- Спершу рухати модулі з мінімальною кількістю cross-slice залежностей.
- Якщо перенос не дає виграшу в читабельності або межах, не робити його “для краси”.

## Черга виконання

1. Зафіксувати цільові межі модулів і naming rules.
2. Винести shared frontend/backend utilities в окремі `shared` зони.
3. Перенести `tasks`.
4. Перенести `reports`.
5. Перенести `counterparties`.
6. Перенести `dashboard` і `shell`.
7. Перенести `settings`.
8. Лише після стабілізації рухати `payments`.
9. Останнім переносити `documents`, `pdf`, `import`.

## Пріоритети за ризиком

### Низький ризик

- `tasks`
- `reports`
- `dashboard`

### Середній ризик

- `counterparties`
- `settings`
- `shell`

### Високий ризик

- `payments`
- `documents`
- `import`
- `pdf`

### Причини високого ризику

- `payments` мають import, matching, reconcile, calendar flow.
- `documents` мають chain flow, PDF flow, bulk actions, кілька типів документів.
- `import` і `pdf` тягнуть інфраструктурні та IO-bound залежності.

### Task 1: Зафіксувати канонічну цільову карту модулів

**Files:**
- Modify: `docs/architecture/app-state.md`
- Modify: `docs/architecture/tauri-command-surface.md`
- Create: `docs/architecture/refactoring-target-structure.md`
- Create: `docs/superpowers/plans/2026-05-04-refactoring-roadmap.md`

- [ ] **Step 1: Описати цільові backend і frontend boundaries**

Додати окремий архітектурний документ з:
- поточною картою шарів;
- цільовою feature-first картою;
- правилами для `shared`, `app`, `integrations`, `documents_io`;
- переліком slice'ів у рекомендованому порядку переносу.

- [ ] **Step 2: Зафіксувати invariant'и, які рефакторинг не має ламати**

Окремо вписати:
- command surface стабільний;
- DTO casing стабільний;
- money contract стабільний;
- active company scoping стабільний;
- screen-local state не переїжджає в backend.

- [ ] **Step 3: Оновити app-state documentation під нову карту**

Уточнити, що store'и, invoke bridge і backend use-cases мають групуватися по feature slice, а не рости окремими технічними деревами.

- [ ] **Step 4: Оновити tauri-command-surface documentation**

Додати примітку, що фізичне розташування команд може змінюватися, але публічний invoke surface лишається канонічним контрактом.

- [ ] **Step 5: Перевірити узгодженість документів**

Перечитати `docs/architecture/app-state.md`, `docs/architecture/tauri-command-surface.md`, `docs/architecture/refactoring-target-structure.md` і звірити, що назви slice'ів однакові в усіх трьох документах.

### Task 2: Винести shared зони без зміни поведінки

**Files:**
- Modify: `src/lib.rs`
- Create: `src/shared/mod.rs`
- Move/Modify: `src/app_ctx.rs`
- Move/Modify: `src/config.rs`
- Move/Modify: `src/runtime.rs`
- Move/Modify: frontend shared utility files around `frontend/src/lib/api.ts`, `frontend/src/lib/types.ts`, `frontend/src/lib/components`, `frontend/src/lib/styles`
- Test: `cargo test`
- Test: `npm run check`
- Test: `npm run test:frontend`

- [ ] **Step 1: Backend shared inventory**

Зафіксувати, які backend файли справді shared:
- `app_ctx.rs`
- `config.rs`
- `runtime.rs`
- money formatting helpers
- cross-slice types

- [ ] **Step 2: Frontend shared inventory**

Зафіксувати, що лишається shared у frontend:
- invoke bridge
- global TS DTO types
- design tokens
- reused UI components
- browser fallback helpers

- [ ] **Step 3: Перенести shared backend files через мінімальні re-export'и**

Зробити перенесення так, щоб старі імпорти тимчасово працювали через `pub use`, поки не буде завершено хвилю стабілізації.

- [ ] **Step 4: Перенести shared frontend files без зміни public import surface**

Спочатку зберегти сумісність через barrel/re-export або тонкі проміжні модулі.

- [ ] **Step 5: Запустити compile/type/test checkpoint**

Run: `cargo test`
Expected: PASS

Run: `npm run check`
Expected: PASS

Run: `npm run test:frontend`
Expected: PASS

### Task 3: Пілотний перенос `tasks`

**Files:**
- Modify/Move: `src/db/tasks.rs`
- Modify/Move: `src/models/task.rs`
- Modify/Move: `src/tauri_api/tasks.rs`
- Modify/Move: `src-tauri/src/commands/tasks.rs`
- Modify/Move: `frontend/src/lib/stores/tasks.ts`
- Modify/Move: `frontend/src/lib/screens/TasksScreen.svelte`
- Modify: `frontend/src/lib/api.ts`
- Modify: `frontend/src/lib/types.ts`
- Test: `tests/tauri_vertical_slice.rs`

- [ ] **Step 1: Створити цільову feature-папку для tasks**

Завести окремі зони для:
- backend domain/query/use-case;
- frontend store/screen/types helpers;
- command wrapper.

- [ ] **Step 2: Перенести backend tasks slice**

Перемістити `db`, `models`, `tauri_api` частини ближче одна до одної, не міняючи командні імена та DTO форму.

- [ ] **Step 3: Перенести frontend tasks slice**

Згрупувати `TasksScreen`, store і пов'язані helpers в одну feature-зону.

- [ ] **Step 4: Прогнати вузькі перевірки**

Run: `cargo test tests::tauri_vertical_slice -- --nocapture`
Expected: relevant tasks flows PASS

Run: `npm run test:frontend`
Expected: tasks-related tests PASS

- [ ] **Step 5: Зафіксувати lessons learned**

Дописати в `docs/architecture/refactoring-target-structure.md`, що спрацювало/не спрацювало на пілоті `tasks`.

### Task 4: Перенести `reports`

**Files:**
- Modify/Move: `src/db/reports.rs`
- Modify/Move: `src/models/reports.rs`
- Modify/Move: `src/tauri_api/reports.rs`
- Modify/Move: `src/tauri_api/reports_excel.rs`
- Modify/Move: `src-tauri/src/commands/reports.rs`
- Modify/Move: `frontend/src/lib/stores/reports.ts`
- Modify/Move: `frontend/src/lib/screens/ReportsScreen.svelte`
- Modify: `frontend/src/lib/api.ts`
- Modify: `frontend/src/lib/types.ts`

- [ ] **Step 1: Перенести core reports slice**

Тримати поруч data loading, filters, export flow і screen/store contract.

- [ ] **Step 2: Відокремити export logic як підмодуль reports**

`reports_excel` лишити підсистемою `reports`, а не окремим горизонтальним винятком.

- [ ] **Step 3: Перевірити top-counterparties flow після переносу**

Окремо прогнати сценарії active tab, filters, export, row focus, drill CTA.

- [ ] **Step 4: Запустити regression checkpoint**

Run: `cargo test`
Expected: PASS

Run: `npm run test:frontend`
Expected: PASS

### Task 5: Перенести `counterparties`

**Files:**
- Modify/Move: `src/db/counterparties.rs`
- Modify/Move: `src/models/counterparty.rs`
- Modify/Move: `src/tauri_api/counterparties.rs`
- Modify/Move: `src-tauri/src/commands/counterparties.rs`
- Modify/Move: `frontend/src/lib/stores/counterparties.ts`
- Modify/Move: `frontend/src/lib/screens/CounterpartiesScreen.svelte`
- Modify: `frontend/src/lib/api.ts`
- Modify: `frontend/src/lib/types.ts`

- [ ] **Step 1: Перенести slice без зміни document-context flow**

`counterparty_create_document_context` лишається canonical boundary між counterparties і documents.

- [ ] **Step 2: Не вмонтовувати documents logic у counterparties**

Під час переносу не змішувати створення документів із власним counterparty slice.

- [ ] **Step 3: Прогнати screen/store/backend тести**

Run: `cargo test`
Expected: PASS

Run: `npm run test:frontend`
Expected: PASS

### Task 6: Перенести `dashboard` і `shell`

**Files:**
- Modify/Move: `src/db/dashboard.rs`
- Modify/Move: `src/actions/inbox.rs`
- Modify/Move: `src/actions/palette.rs`
- Modify/Move: `src/tauri_api/dashboard.rs`
- Modify/Move: `src-tauri/src/commands/dashboard.rs`
- Modify/Move: `src-tauri/src/commands/shell.rs`
- Modify/Move: `frontend/src/App.svelte`
- Modify/Move: `frontend/src/lib/stores/app-shell.ts`
- Modify/Move: `frontend/src/lib/stores/navigation.ts`
- Modify/Move: `frontend/src/lib/stores/palette.ts`
- Modify/Move: `frontend/src/lib/stores/shell.ts`

- [ ] **Step 1: Виділити shell як окремий feature-root**

Shell має містити:
- app bootstrap contract;
- active company switching;
- palette;
- top-level navigation;
- shell chrome.

- [ ] **Step 2: Не змішувати shell з dashboard**

`dashboard` лишається окремим бізнес-slice, навіть якщо UX живе на домашньому екрані.

- [ ] **Step 3: Прогнати keyboard/accessibility regression**

Run: `npm run test:frontend`
Expected: App shell, palette, dashboard tests PASS

### Task 7: Перенести `settings`

**Files:**
- Modify/Move: `src/db/companies.rs`
- Modify/Move: `src/tauri_api/settings.rs`
- Modify/Move: `src-tauri/src/commands/settings.rs`
- Modify/Move: `frontend/src/lib/stores/settings.ts`
- Modify/Move: `frontend/src/lib/screens/SettingsScreen.svelte`

- [ ] **Step 1: Розділити settings і shared config boundaries**

UI/settings flow має бути окремим feature slice, а persistence/config primitives — у shared.

- [ ] **Step 2: Не тягнути company persistence в shell**

Компанії залишаються в settings/domain зоні, shell лише переключає active company.

- [ ] **Step 3: Прогнати settings/import smoke regression**

Run: `cargo test`
Expected: PASS

Run: `npm run test:frontend`
Expected: PASS

### Task 8: Перенести `payments`

**Files:**
- Modify/Move: `src/db/payments.rs`
- Modify/Move: `src/models/payment.rs`
- Modify/Move: `src/services/payment_matching.rs`
- Modify/Move: `src/import/bank_common.rs`
- Modify/Move: `src/import/bank_csv.rs`
- Modify/Move: `src/import/bank_xlsx.rs`
- Modify/Move: `src/tauri_api/payments.rs`
- Modify/Move: `src-tauri/src/commands/payments.rs`
- Modify/Move: `frontend/src/lib/stores/payments.ts`
- Modify/Move: `frontend/src/lib/components/PaymentCalendarPanel.svelte`
- Modify/Move: `frontend/src/lib/screens/PaymentsScreen.svelte`

- [ ] **Step 1: Спершу виділити внутрішні підсистеми payments**

Розвести окремо:
- listing/editor;
- reconcile/matching;
- calendar/schedule;
- bank import adapters.

- [ ] **Step 2: Лише потім фізично переносити папки**

Без попереднього розділення внутрішніх зон `payments` легко перетвориться на новий великий моноліт.

- [ ] **Step 3: Прогнати розширений regression checkpoint**

Run: `cargo test`
Expected: PASS

Run: `npm run test:frontend`
Expected: PASS

Run: `npm --prefix e2e-tests test`
Expected: smoke e2e PASS

### Task 9: Перенести `documents`, `pdf`, `import`

**Files:**
- Modify/Move: `src/db/acts.rs`
- Modify/Move: `src/db/invoices.rs`
- Modify/Move: `src/db/waybills.rs`
- Modify/Move: `src/db/document_templates.rs`
- Modify/Move: `src/models/act.rs`
- Modify/Move: `src/models/invoice.rs`
- Modify/Move: `src/models/waybill.rs`
- Modify/Move: `src/models/document_template.rs`
- Modify/Move: `src/pdf/generator.rs`
- Modify/Move: `src/pdf/reader.rs`
- Modify/Move: `src/tauri_api/documents.rs`
- Modify/Move: `src-tauri/src/commands/documents.rs`
- Modify/Move: `frontend/src/lib/stores/documents.ts`
- Modify/Move: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Step 1: Перед переносом розбити documents на внутрішні модулі**

Рекомендовані підзони:
- `documents/core`
- `documents/chain`
- `documents/pdf`
- `documents/templates`
- `documents/editor`

- [ ] **Step 2: Визначити місце для PDF**

PDF або лишається під `documents_io/pdf`, або входить у `documents/pdf`, якщо залежність лишається суто document-centric.

- [ ] **Step 3: BAS import не змішувати з documents core**

BAS adapters мають жити або в `integrations/bas`, або в окремому import-root, навіть якщо вони створюють документи.

- [ ] **Step 4: Прогнати повний regression checkpoint**

Run: `cargo test`
Expected: PASS

Run: `npm run check`
Expected: PASS

Run: `npm run test:frontend`
Expected: PASS

Run: `npm --prefix e2e-tests test`
Expected: PASS

## Швидка рефакторинг-мапа по папках

### Варто об'єднати по feature

- `src/db/tasks.rs` + `src/models/task.rs` + `src/tauri_api/tasks.rs`
- `src/db/reports.rs` + `src/models/reports.rs` + `src/tauri_api/reports.rs`
- `src/db/counterparties.rs` + `src/models/counterparty.rs` + `src/tauri_api/counterparties.rs`
- `frontend/src/lib/stores/tasks.ts` + `frontend/src/lib/screens/TasksScreen.svelte`
- `frontend/src/lib/stores/reports.ts` + `frontend/src/lib/screens/ReportsScreen.svelte`
- `frontend/src/lib/stores/counterparties.ts` + `frontend/src/lib/screens/CounterpartiesScreen.svelte`

### Варто лишити shared

- `AppCtx`
- `runtime`
- global money formatting helpers
- invoke wrapper
- TS DTO contract primitives
- shared UI components
- design tokens

### Варто відкласти до кінця

- `payments`
- `documents`
- `pdf`
- `import`

## Критерії успіху

- Новий розробник може знайти весь код slice'а в 1-2 суміжних зонах, а не в 5 різних деревax.
- Публічний Tauri command surface не змінився без причини.
- Rust compile, frontend type-check і тести проходять після кожної хвилі.
- `payments` і `documents` не перетворені на нові “mega-modules”.
- Архітектурні документи відображають нову реальність, а не історичний намір.

## Рекомендований порядок виконання

1. `shared` boundaries
2. `tasks`
3. `reports`
4. `counterparties`
5. `dashboard`
6. `shell`
7. `settings`
8. `payments`
9. `documents`
10. `pdf/import` stabilization

## Self-review

- Spec coverage: план покриває target structure, порядок хвиль, межі shared, пріоритети ризику, папки для переносу і compile/test checkpoints.
- Placeholder scan: залишено лише ті кроки, які можна виконати як окремі фази без вигаданих деталей реалізації.
- Type consistency: назви slice'ів узгоджені між поточною картою, target structure і чергою виконання.
