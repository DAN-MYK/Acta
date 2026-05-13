# План виправлень — Code Review гілки `codex/p1-ui-polish-followup`

**Дата:** 2026-05-05
**Скоуп:** усі знахідки 5-ти ревюерів (Rust backend / Svelte components+stores / Screens / Tests / CI+build) для гілки vs `main`.

---

## Принципи виконання

- Виконувати **по фазах**: Phase 0 (CI блокери) → Phase 1 (фінансова коректність + безпека) → решта.
- Кожен пункт самодостатній: `file:line` + рекомендована дія + effort estimate.
- Тести — спочатку failing test, потім фікс (TDD), для всіх корекційних правок money/payments/security.
- Після кожної фази: `cargo build --tests` + `npm run test:frontend` + `svelte-check`.
- Money fix (P1.1) має передувати всім іншим змінам у `payments.ts/paymentsUtils.ts`, бо вплине на API.

---

## Phase 0 — CI блокери (фіксити негайно, 30 хв)

### P0.1 — Закомітити `scripts/check-text-encoding.mjs`
- **Проблема:** скрипт untracked, але `package.json:11` і `.github/workflows/ci.yml:48-49` його дзвонять. Перший push зламає CI.
- **Дія:** `git add scripts/check-text-encoding.mjs` + перевірити логіку (UTF-8 BOM, CRLF/LF detection).
- **Effort:** 5 хв.

### P0.2 — Виправити mojibake у E2E специфікаціях
- **Файли:** `e2e-tests/test/specs/app-smoke.e2e.js:81, 99`.
- **Проблема:** UTF-8 декодований як CP1251 і пере-енкодений → `"РџРµСЂРµРјРёРєР°С‡..."`.
- **Дія:** відновити рядки `"Перемикач теми не змінив body[data-theme] у native Tauri runtime"` і `"Тема не повернулась у початковий стан після smoke toggle"`.
- **Effort:** 5 хв.

### P0.3 — Розширити encoding-guard на E2E теку
- **Файл:** `scripts/check-text-encoding.mjs:6` (`scanRoots`).
- **Дія:** додати `"e2e-tests/test"` до scanRoots; розширити mojibake regex до `/(?:[РСÐÑ][-ÿ]){3,}/`.
- **Effort:** 10 хв.

### P0.4 — Повернути `paths-ignore` для `pull_request`
- **Файл:** `.github/workflows/ci.yml:10-12`.
- **Дія:** додати `pull_request: paths-ignore: ["docs/**", "**/*.md"]`.
- **Effort:** 2 хв.

---

## Phase 1 — Критичні фінансові та безпекові діркі

### P1.1 — Money pipeline на bigint (копійки)

**Корінь:** `parseFloat` для розподілу платежів порушує money-contract з CLAUDE.md.

#### P1.1.1 — Винести спільні bigint-helpers
- **Новий файл:** `frontend/src/lib/money.ts`.
- **Експортувати:** `parseMoneyToMinor(s: string): bigint | null`, `formatMinorMoney(minor: bigint): string`, `addMinor(...vals)`, `subMinor(a, b)`, `compareMinor(a, b)`.
- **Source:** перевикористати логіку з `documentMoney.ts:18-91`.
- **Effort:** 1 год.
- **Тести:** `lib/__tests__/money.test.ts` — edge cases NaN, Infinity, 1e21, від'ємні, NBSP, кома/крапка.

#### P1.1.2 — Переписати `paymentsUtils.ts`
- **Файл:** `frontend/src/lib/stores/paymentsUtils.ts:38-47`.
- **Дія:** `parseMoneyValue → parseMoneyToMinor`, `formatMoneyValue` через bigint. Експортувати обидва.
- **Effort:** 30 хв.

#### P1.1.3 — Переписати арифметику в payments store
- **Файл:** `frontend/src/lib/stores/payments.ts:329-339, 348-352, 368-376, 893-900, 941-984, 1080`.
- **Дія:** замінити `parseFloat`/`Math.min` на bigint compare/sub/min. `confirmSplitDraft` валідує `remainingMinor === 0n` (точна рівність).
- **Effort:** 1.5 год.

#### P1.1.4 — Об'єднати дублі в reports
- **Файли:** `frontend/src/lib/screens/ReportsScreen.svelte:131-135` + `frontend/src/lib/stores/reports.ts:49-53`.
- **Дія:** видалити локальні `parseMoneyValue`, імпортувати з `money.ts`.
- **Effort:** 15 хв.

### P1.2 — Закрити over-allocation діркі в платежах

#### P1.2.1 — Додати FOR UPDATE на document rows у split-reconcile
- **Файл:** `src/db/payments.rs:341-438` (`reconcile_split_scoped`).
- **Дія:** перед перевіркою available для кожного allocation робити `SELECT 1 FROM acts/invoices WHERE id=$1 AND company_id=$2 FOR UPDATE`.
- **Effort:** 1 год.
- **Тести:** `tests/db_integration/payments.rs` — concurrent reconcile двох платежів на один документ.

#### P1.2.2 — Видалити single-doc fast path або зробити його через split
- **Файл:** `src/db/payments.rs:207-261` (`reconcile_document_scoped`).
- **Дія:** перетворити на адаптер що формує `[allocation]` і викликає `reconcile_split_scoped`. Видалити прямі `INSERT ... ON CONFLICT` без перевірки інваріантів.
- **Effort:** 45 хв.

#### P1.2.3 — `payment_match_apply_auto` через транзакцію
- **Файл:** `src/tauri_api/payments.rs:1064-1103`.
- **Дія:** замість `compute_match_preview → reconcile_document_scoped` поза транзакцією, побудувати `[PaymentReconcileAllocation]` з preview і викликати `reconcile_split_scoped` атомарно.
- **Effort:** 30 хв.

### P1.3 — Path traversal у PDF flow

#### P1.3.1 — Зберігати relative PDF paths
- **Файли:** `src/db/invoices.rs:174-197`, `src/db/waybills.rs:185-209`, `src/tauri_api/documents.rs::persist_existing_pdf_path`.
- **Дія:** persist `existing_pdf/{kind}/{uuid}_{slug}/working.pdf` як relative; join з `ctx.storage_dir()` при читанні.
- **Effort:** 1.5 год.
- **Міграція:** `migrations/<ts>_relativize_pdf_paths.sql` що приводить існуючі absolute → relative (strip `storage_dir()` prefix).

#### P1.3.2 — Canonicalize + assert under storage_dir у attach/replace
- **Файли:** `src/tauri_api/documents.rs:242-283` (attach), `:414-450` (replace).
- **Дія:**
  - validate `source_path.extension() == Some("pdf")`;
  - `tokio::fs::canonicalize(source_path)` → assert що result не всередині `ctx.storage_dir()/existing_pdf/` (запобігання loop self-overwrite);
  - `replace`: canonicalize result з DB, assert що path під `managed_existing_pdf_dir(...)`.
- **Effort:** 1 год.

### P1.4 — TOCTOU у import commit
- **Файл:** `src/tauri_api/payments.rs:1042-1085`.
- **Дія:** опція А (мінімальна) — `tokio::fs::read(path).await?` один раз, передати `Vec<u8>` парсеру; опція Б — хешувати bytes на preview, зберегти hash у `ImportPreviewSnapshot`, на commit пере-хешувати і порівняти.
- **Effort:** 1.5 год (опція Б).
- **Тести:** integration з модифікацією файлу між preview і commit.

### P1.5 — Виправити тихе ковтання в `parse_event_amount`
- **Файл:** `src/tauri_api/payments.rs:344-350`.
- **Дія:** додати поле `amount: Decimal` до `PaymentCalendarEventDto` (з `#[serde(skip)]`), використовувати у `income_total`/`expense_total`. Видалити `parse_event_amount`.
- **Effort:** 30 хв.

### P1.6 — Винести dead-tests з `vi.hoisted()`

#### P1.6.1 — PaymentsScreen test
- **Файл:** `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts:74-287`.
- **Дія:** перенести 5 `it()` блоків (split-draft mojibake, split-allocation, manual-picker, recommended candidates, manual confirmation aria-describedby) **за межі** hoisted-callback. Підтвердити: `vitest --reporter=verbose` показує 18 тестів (було 13).
- **Effort:** 20 хв.

#### P1.6.2 — SettingsScreen test
- **Файл:** `frontend/src/lib/screens/__tests__/SettingsScreen.test.ts:74-102`.
- **Дія:** аналогічно перенести 2 тести (segmented control + radio semantics). 14 тестів замість 12.
- **Effort:** 10 хв.

#### P1.6.3 — Захист від recurrence
- **Файл:** `tsconfig.json`.
- **Дія:** додати `"allowUnreachableCode": false`, `"noFallthroughCasesInSwitch": true`.
- **Effort:** 5 хв.

### P1.7 — Drawer focus trap + unsaved-changes guard

#### P1.7.1 — `inert` атрибут на основну панель
- **Файли:** `frontend/src/lib/screens/DocumentsScreen.svelte:335` (panel), `TasksScreen.svelte`, `CounterpartiesScreen.svelte`, `PaymentsScreen.svelte` editor.
- **Дія:** `inert={$store.editor ? "" : undefined}` на головній `<section class="panel">`.
- **Effort:** 30 хв.

#### P1.7.2 — Dirty flag + confirmCloseIfDirty
- **Стори:** `documents.ts`, `counterparties.ts`, `payments.ts`, `tasks.ts`.
- **Дія:**
  - при `openEditor` зберегти snapshot `editorSnapshot`;
  - експортувати `isEditorDirty(): boolean` (порівняння поточного state зі snapshot);
  - `closeEditor(force = false)` — якщо dirty і не force → повернути `{ ok: false, reason: "dirty" }`.
- **UI:** у DocumentsScreen/TasksScreen/CounterpartiesScreen/PaymentsScreen усі точки виходу (ESC, бекдроп, кнопка Закрити) попередньо викликають `confirmCloseIfDirty()` — інлайн-banner у drawer header (не `window.confirm`, бо блокує screen reader).
- **Effort:** 3 год.
- **Тести:** для кожного screen — заповнити форму, спробувати ESC → assert drawer не закрився.

### P1.8 — CSS токени Wave 1: alias-міграція

**Корінь:** Wave 1 design migration оголошена як завершена (commit `2de0a940`), але багато компонентів і екранів ще на legacy токенах (`--bg-card`, `--accent`, `--space-N`, `--floating-shadow`, `--positive-strong`).

#### P1.8.1 — Додати alias-токени у `tokens.css`
- **Файл:** `frontend/src/lib/styles/tokens.css`.
- **Дія:** у блоці `:root` додати `--space-1: var(--acta-space-1)` … `--space-6`, `--floating-shadow: var(--acta-shadow-card)`, `--positive-strong: var(--acta-color-success)`, `--bg-card: var(--acta-color-surface)`, `--accent: var(--acta-color-primary)`, `--accent-soft: var(--acta-color-primary-soft)`, `--text-muted: var(--acta-color-text-muted)`, `--border-hairline: var(--acta-color-border)`, `--danger: var(--acta-color-danger)`. Перевірити що всі мають значення в light і dark.
- **Effort:** 1 год.
- **Перевірка:** `grep -r "var(--[a-z]" frontend/src --include="*.css" --include="*.svelte" | grep -v "var(--acta-"` — список legacy. Кожен має мати alias.

#### P1.8.2 — Чистка legacy використань (incremental)
- **Дія:** у наступному PR — замінити `var(--bg-card)` на `var(--acta-color-surface)` тощо у компонентах. Не блокує merge цієї гілки.
- **Effort:** 4 год (окремий PR).

### P1.9 — Дублі типів у `types.ts`
- **Файл:** `frontend/src/lib/types.ts:370-389, 392-399, 493-507`.
- **Дія:** видалити повторні декларації `PaymentReconcileSplitAllocationRequest`, `PaymentReconcileSplitRequest`, `PaymentReconcileSplitResultDto`, `PaymentReconcileSplitAllocationResultDto`. Залишити одну версію кожного, що відповідає поточному backend DTO.
- **Effort:** 30 хв (треба звірити з `src/tauri_api/payments.rs` структурами).

---

## Phase 2 — Warnings (бажано перед merge)

### Backend Rust

| ID | File:line | Дія | Effort |
|----|-----------|-----|--------|
| W1 | `src/services/payment_matching.rs:275-344` | Або відкотити дозвіл `same_iban`/`reference_hit` для partial-amount, або задокументувати + додати тест на scoring constants | 45 хв |
| W2 | `src/db/reports.rs:912-951` | Застосувати `query` filter також у `compute_opening_balance` для consistency `closing = opening + Σ filtered_rows` | 30 хв |
| W3 | `src/db/reports.rs:56-66` | Unit тест на `query_like_pattern` що пінить точні байти для `"a\b"`, `"50%"` | 20 хв |
| W4 | `src/db/reports.rs:140-146` | Перевірити GROUP BY composition для `"Без контрагента"` collision; додати `cp.id` до GROUP BY | 30 хв |
| W5 | `src/tauri_api/payments.rs:686-760` | N+1 → batch fetch counterparties: `list_schedule_in_range_with_counterparty` через JOIN | 1 год |
| W6 | `src/import/bank_xlsx.rs:1188-1196` | Doc comment: функція коректна для serial ≥ 60 (1900-03-01); або skip < 60 з warning | 15 хв |
| W7 | `src-tauri/src/commands/payments.rs:155-176` | `blocking_pick_file` загорнути в `tauri::async_runtime::spawn_blocking` | 20 хв |
| W8 | `src/pdf/reader.rs:447-504` | `replace_pdf_text_with_report` — кешувати `Document::load` між inspect/replace | 30 хв |
| W9 | `src/tauri_api/reports.rs:1232` | `if let Ok(Err(error))` → match на всі три варіанти (`Ok(Ok)`, `Ok(Err)`, `Err`) з логуванням | 15 хв |
| W10 | `src/tauri_api/payments.rs:474-516` | `tokio::join!` для незалежних futures у `build_match_input` | 15 хв |
| W11 | `src/services/payment_matching.rs:225-273` | Винести `MAX_SPLIT_CANDIDATES: usize = 6` як константа з doc-comment | 10 хв |
| W12 | `src/db/payments.rs:449` (`link_invoice`) | Перевірити usages, видалити якщо dead | 10 хв |

### Frontend компоненти/stores

| ID | File:line | Дія | Effort |
|----|-----------|-----|--------|
| W13 | `frontend/src/lib/components/Modal.svelte:30-44` | Focus trap охоплює `[contenteditable]`, media controls; `body.style.overflow` lock; `previousActiveElement.focus()` через `tick()` | 1 год |
| W14 | `frontend/src/lib/components/CommandBar.svelte:14-17` | Debounce 200мс на `dispatch('search', value)` | 20 хв |
| W15 | `frontend/src/lib/components/Table.svelte:5` | Generic `<T extends Record<string, unknown>>` через `<script lang="ts" generics="T">` | 30 хв |
| W16 | `frontend/src/lib/components/KPI.svelte:6,17-20` | `value: number` → `value: string` (приходить вже відформатоване з backend); UI лише delta-знак | 30 хв |
| W17 | `frontend/src/lib/components/SkeletonCard.svelte`, `SkeletonRow.svelte` | `role="status" aria-live="polite" aria-busy="true"` + visually-hidden "Завантажуємо…" | 20 хв |
| W18 | `frontend/src/lib/stores/payments.ts:648-698, 786-825` | RequestId guard у `reconcile`/`openManualMatchPicker` (паттерн `latestLoadRequestId` з reports.ts) | 1 год |
| W19 | `frontend/src/lib/stores/shell.ts:60-68` | Замінити рекурсію `this.load()` на `while` цикл | 20 хв |
| W20 | `frontend/src/lib/stores/palette.ts:122-133` | `import { get } from "svelte/store"` замість subscribe+unsubscribe | 5 хв |
| W21 | `frontend/src/lib/components/PaymentCalendarPanel.svelte:117-128` | `role="tablist"` → `role="radiogroup"` (це segmented control, не tab) | 20 хв |
| W22 | `frontend/src/lib/components/PaymentCalendarPanel.svelte:180-187` | `aria-label={day.date} ({eventCount} подій)` на day buttons | 15 хв |
| W23 | `frontend/src/lib/components/FormField.svelte:10` | `for={id ?? undefined}` (не emit `for=""`); setContext для `descriptionId` | 30 хв |
| W24 | `frontend/src/lib/stores/documents.ts` | `activeAction` per-mutation guard замість одного `loading` flag | 1 год |
| W25 | `frontend/src/App.svelte:71-99` + Modal.svelte | Винести focus trap у `lib/utils/focusTrap.ts` (Svelte action), використати в обох | 1.5 год |

### Screens

| ID | File:line | Дія | Effort |
|----|-----------|-----|--------|
| W26 | `frontend/src/lib/screens/PaymentsScreen.svelte:300-318` | Кнопка `Перечитати файл` → `payments.refreshImportPreview()`; auto-set `state.importPreviewStale` при fingerprint mismatch | 1 год |
| W27 | `DocumentsScreen.svelte:157`, `CounterpartiesScreen.svelte:11`, `ReportsScreen.svelte:38` | Debounce 250мс на search input; requestId guard у store | 1.5 год |
| W28 | `ReportsScreen.svelte:56-86` | WAI-ARIA tabs: arrows тільки міняють focus, Enter/Space активують | 30 хв |
| W29 | `ReportsScreen.svelte:187-198` | `daysUntil` рахувати локально як `paymentsUtils.ts:27-36`; винести в `lib/dates.ts` | 30 хв |
| W30 | `CounterpartiesScreen.svelte:401-405` | `inputmode="numeric" pattern="[0-9]{8,10}" maxlength="10"` для ЄДРПОУ; preview-перевірка дублю; `disabled={!form.name}` на Зберегти | 45 хв |
| W31 | `PaymentsScreen.svelte:188-191` | Inverted boolean fix у `manualPickerDisabledReason` | 10 хв |
| W32 | `PaymentsScreen.svelte:627,673`, `ReportsScreen.svelte:589,*` | `data-negative` через regex `/^[-(]/` (підтримка дужок); ідеально — backend повертає `negative: boolean` | 30 хв |
| W33 | `CounterpartiesScreen.svelte:144-146`, `DashboardScreen.svelte:43-45` | Уніфікувати з `SettingsScreen.svelte:79-104` патерном `status-banner` (`role="status"` для loading, `role="alert"` для error) | 30 хв |
| W34 | `PaymentsScreen.svelte:569-575` | `inputmode="decimal" placeholder="0,00"` + inline-error під полем allocation | 45 хв |
| W35 | `SettingsScreen.svelte:13-14, 316-329` | Reactive cleanup `showBasImport=false; importBas.reset()` при section change | 15 хв |
| W36 | `SettingsScreen.svelte:29-34` | Disable theme buttons до завершення `savePreferences` | 15 хв |

### Tests

| ID | File:line | Дія | Effort |
|----|-----------|-----|--------|
| W37 | `tests/db_integration/reports.rs` (49× `DEFAULT_COMPANY_ID`) | Перейти на per-test company helper як у `tests/db_integration/dashboard.rs:3-38`; cascade DELETE by company_id | 4 год |
| W38 | `tests/db_integration/payments.rs:147-225, 230-390` | Паттерн `let result: Result<()> = async {…}.await; cleanup; result?;` (з `tauri_vertical_slice.rs:101-257`) | 2 год |
| W39 | `frontend/src/lib/screens/__tests__/CounterpartiesScreen.test.ts:249` | `confirmSpy.mockRestore()` після assertion | 5 хв |
| W40 | Frontend `__tests__/*` (155× data-testid) | Поступово мігрувати на `getByRole`/`getByLabelText` для нових тестів; legacy не чіпати | поточний |
| W41 | `tests/tauri_vertical_slice.rs:619-620` | `Utc::now().date_naive() - Duration::days(365 * 5)` замість hardcoded 2000-01-01 | 10 хв |
| W42 | `tests/tauri_vertical_slice.rs:42, 593, 699` | `tempfile::TempDir` замість `storage/test-config`; уникнути `set_var` race | 1 год |
| W43 | `frontend/src/lib/stores/__tests__/dashboard.test.ts:112` | Видалити `}, 10000);` (cargo cult timeout) | 5 хв |

### CI/build

| ID | File:line | Дія | Effort |
|----|-----------|-----|--------|
| W44 | `.github/workflows/ci.yml:37,40,70,73,93,96,113,122,125,186,189` | Pin actions на commit SHA з коментарем тегу; додати Dependabot config | 1 год |
| W45 | `e2e-tests/tauri-build-coordinator.js:185-217` | Після циклу `runBuildWithRetry` додати `throw new Error("Tauri build did not converge after retries")` | 5 хв |
| W46 | `e2e-tests/tauri-build-coordinator.js:222` | `stdio: "pipe"` → streaming через async `spawn` або `stdio: ['ignore','inherit','inherit']` | 1 год |
| W47 | `e2e-tests/tauri-build-coordinator.js:173-184` | Видалити невикористані параметри `stampPath`, `dependencyPaths` або реалізувати їх | 10 хв |
| W48 | `tsconfig.json:12` | Винести `"vitest/globals"` у `vitest.tsconfig.json` через `references` (не leak у app код) | 30 хв |
| W49 | `scripts/ensure-vite-dev.mjs:5-7,42-53` | Probe deterministic `${devUrl}/@vite/client` замість `<title>Acta</title>` substring | 30 хв |
| W50 | `.github/workflows/ci.yml:99` | Cache `~/.cargo/bin/sqlx` через `cargo-binstall` або `actions/cache` | 30 хв |
| W51 | `.github/workflows/ci.yml:196-201` | Видалити манульний `pg_isready` loop (service container healthcheck вже гарантує); або додати `|| (echo …; exit 1)` | 10 хв |
| W52 | `e2e-tests/test/specs/app-smoke.e2e.js:43-58`, `reports-documents-smoke.e2e.js:71-94` | Видалити assertions точних CSS значень (`flexDirection`, `gridTemplateColumns`); залишити `hasHorizontalOverflow === false` | 30 хв |

---

## Phase 3 — Info / tech-debt (окремий PR)

| ID | File:line | Дія | Effort |
|----|-----------|-----|--------|
| I1 | `src/db/reports.rs` (8 `Row { … }` блоків) | Витягти спільний макрос або `query!` з aliasing | 2 год |
| I2 | `src/services/payment_matching.rs:37-79` | `MatchCandidate::act/invoice` через єдиний приватний `new(kind, …)` | 30 хв |
| I3 | `src/models/reports.rs:30` | Видалити невикористаний alias `PnlCategoryRow` | 5 хв |
| I4 | `frontend/src/lib/screens/ReportsScreen.svelte:131-185` | Винести sort/parse helpers у `lib/reports-utils.ts`, переімпортувати з обох | 1 год |
| I5 | `frontend/src/lib/styles/tokens.css:133-142` | Розбити `--acta-text-h1: 24px / 30px var(--acta-font-sans)` на size/line/weight | 30 хв |
| I6 | `frontend/src/lib/screens/SettingsScreen.svelte:140-163` | Видалити `{#if false}` dead code | 5 хв |
| I7 | `frontend/src/lib/icons/index.ts` | Профайл bundle (`vite build --analyze`); lazy-load рідких іконок | 1 год |
| I8 | `frontend/src/lib/__tests__/document-money.test.ts` + `payments-utils.test.ts` | Edge cases: NaN, Infinity, 1e21, від'ємні quantity, NBSP/ASCII space | 1 год |
| I9 | NBSP vs ASCII у frontend tests | Стандартизувати на `normalizeMoneyText` helper або точний ` ` всюди | 30 хв |
| I10 | `tests/db_integration/reports.rs` (фіксти) | Helper `create_test_counterparty_in_company(...)` — видалити 7-`None` boilerplate (~40 місць) | 1.5 год |
| I11 | `tests/db_integration/payments.rs:843-994` | Параметризовані sub-test для validation matrix `reconcile_split_scoped` | 1 год |
| I12 | `tests/db_integration/payments.rs:1093-1142` | Додати кейс `[invalid_allocation, valid_allocation]` ordering у atomicity test | 30 хв |
| I13 | `frontend/src/lib/screens/DocumentsScreen.svelte:253-299` | `getDocumentKindIcon`/`Label` через exhaustive switch на `DocumentKind` enum | 30 хв |
| I14 | `frontend/src/lib/screens/CounterpartiesScreen.svelte:184,196,246,371` | Inline `style="margin: 28px;"` → CSS class | 15 хв |
| I15 | `src/tauri_api/payments.rs::payments_import_preview/commit` | Кеш parsed rows у пам'яті за (path, size, mtime, hash) — уникнути doubled parse | 1 год |
| I16 | `frontend/src/lib/components/Button.svelte:6` | `interface $$Props extends HTMLButtonAttributes` для compile-time перевірки restProps | 20 хв |
| I17 | `frontend/src/lib/components/Modal.svelte:75,78` | Замінити статичний `id="modal-title"` на `crypto.randomUUID()`-based | 15 хв |
| I18 | `Cargo.toml` (новий `rust_xlsxwriter`) | Додати CI крок `cargo audit` (rustsec/audit-check@v1) щотижня | 20 хв |

---

## Послідовність виконання (рекомендована)

1. **Phase 0 (~1 год)** — фікс CI блокерів. PR #1.
2. **Phase 1.1 (Money) + 1.6 (dead tests) + 1.9 (duplicate types)** — окремий PR #2 (frontend-only, ~6 год).
3. **Phase 1.2 + 1.3 + 1.4 + 1.5 (Backend correctness/security)** — PR #3 (~7 год). Окремо тестувати DB.
4. **Phase 1.7 (drawer guards) + 1.8 (CSS aliases)** — PR #4 (~5 год).
5. **Phase 2 warnings** — згрупувати по доменах (backend / frontend / screens / tests / CI) у 5 PR-ів. Загалом ~25 год.
6. **Phase 3 info** — окремі PR-и за бажанням, поза critical path.

**Загальна оцінка:** ~45 год активної роботи (Phase 0–2). Phase 3 — додатково ~12 год.

---

## Метрики готовності перед merge

- [ ] `npm run check:encoding` зелений (без mojibake).
- [ ] `cargo build --tests` без warnings.
- [ ] `cargo test` усі зелені — включно з новими concurrent reconcile тестами.
- [ ] `npm run test:frontend` — кількість тестів зросла з 13→18 (Payments) і 12→14 (Settings).
- [ ] `svelte-check`: 0 errors, 0 warnings.
- [ ] `npm run build` без warnings.
- [ ] Manual UAT: split-reconcile 1000.00→333.33+333.33+333.34 успішно проходить (smoke на money fix).
- [ ] Manual UAT: drawer ESC з заповненою формою показує prompt про unsaved changes.
- [ ] Manual UAT: bank import — повторне натискання commit після зміни файлу показує `Перечитати файл`, а не infinite-loop помилок.
- [ ] CI на push: всі джоби зелені.
