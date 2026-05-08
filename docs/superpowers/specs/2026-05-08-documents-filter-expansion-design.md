# Design — Розширений фільтр документів

**Дата:** 2026-05-08
**Гілка-старт:** `codex/p1-ui-polish-followup`
**Скоуп:** `frontend/src/lib/screens/DocumentsScreen.svelte`, `documents` store, `documents` Tauri DTO/API, `db::{acts,invoices,waybills}::list_filtered`.

## Мотивація

Поточна вкладка **Документи** має лише два інструменти фільтрації списку:
- текстовий input `Пошук документів`;
- кнопка `Фільтр`, що відкриває панель з єдиним полем — селект `Контрагент`.

Цього мало для роботи бухгалтера: немає фільтрів за періодом, статусом, сумою, немає швидких пресетів. Backend (`db::*::list_filtered`) уже **частково готовий** — приймає `date_from`, `date_to`, `status_filter` (одиничний), але UI цих параметрів не використовує. Сума не підтримується ні UI, ні backend.

## Цілі

1. Прибрати з UI поле `Пошук документів` повністю (включно зі state та обробником).
2. Розширити панель фільтра: **Період** (від/до + швидкі дата-пресети всередині панелі), **Статус** (multi-select), **Контрагент** (як зараз), **Сума** (від/до).
3. Додати **рядок швидких пресетів** над toolbar: `Усі`, `Чернетки`, `Неоплачені`, `Прострочені`, `Цього місяця`.
4. Додати **рядок чипів активних фільтрів** з кнопкою `×` для миттєвого зняття конкретного фільтра.
5. Додати **badge-лічильник** на кнопці `Фільтр · N`.
6. Розширити backend: multi-status, `amount_min`/`amount_max`, `overdue_only`.

## Не-цілі

- Користувацькі saved-filters (з localStorage / БД). Тільки вбудовані пресети.
- Зміна layout списку документів, нав-табів, kind-chips, drawer-редактора.
- Pagination або infinite-scroll. Нічого крім фільтрації.
- Текстовий пошук — прибраний з UI; backend `query` лишається в DTO як опційний на майбутнє, але frontend його не передає.

## Layout (ASCII)

```
┌────────────────────────────────────────────────────────────────────┐
│  [ Усі ] [ Вихідні ] [ Вхідні ]                                    │  nav-tabs (без змін)
├────────────────────────────────────────────────────────────────────┤
│  [Усі типи] [Рахунки] [Акти] [Накладні]                            │  kind chips (без змін)
├────────────────────────────────────────────────────────────────────┤
│  Швидкі: [Усі] [Чернетки] [Неоплачені] [Прострочені] [Цей місяць]  │  НОВЕ — presets row
├────────────────────────────────────────────────────────────────────┤
│  [ Фільтр · 3 ▾ ]  [ Очистити ]                                    │  toolbar (без input)
├────────────────────────────────────────────────────────────────────┤
│  Активні: [Період: 01.04–08.05 ×] [Статус: Чернетка, Виставлено ×] │  НОВЕ — chips активних
│           [Контрагент: ТОВ Ромашка ×] [Сума: 1 000 – 50 000 ×]     │
├────────────────────────────────────────────────────────────────────┤
│  ┌── filtersOpen=true ──────────────────────────────────────────┐  │
│  │  Період                                                      │  │
│  │   [ Сьогодні ] [ Тиждень ] [ Місяць ] [ Квартал ] [ Рік ]    │  │
│  │   Від [ 2026-04-01 ]   До [ 2026-05-08 ]                     │  │
│  │                                                              │  │
│  │  Статус (multi)                                              │  │
│  │   [✓ Чернетка] [✓ Виставлено] [ Підписано] [ Оплачено]       │  │
│  │   [ Доставлено]                                              │  │
│  │                                                              │  │
│  │  Контрагент                                                  │  │
│  │   [ select: Усі контрагенти ▾ ]                              │  │
│  │                                                              │  │
│  │  Сума, грн                                                   │  │
│  │   Від [ 1000,00 ]   До [ 50000,00 ]                          │  │
│  │                                                              │  │
│  │   [ Скинути ]                       [ Застосувати ]          │  │
│  └──────────────────────────────────────────────────────────────┘  │
├────────────────────────────────────────────────────────────────────┤
│  documents-create-bar / bulk-actions / documents-list (без змін)   │
└────────────────────────────────────────────────────────────────────┘
```

## Поведінка

### Швидкі пресети (topline)
- Клік по чіпу пресета — встановлює одразу комбінацію `(dateFrom, dateTo, statusFilter, overdueOnly)`. `activePresetId` = id обраного. Один-із-N (radio-семантика).
- Чіп `[Усі]` — повний reset фільтрів і `activePresetId = "all"`.

### Скидання активного пресета
- Будь-яка ручна зміна `dateFrom/dateTo/statusFilter/amountMin/amountMax/counterpartyFilterId` після вибору пресета **скидає `activePresetId = null`** (підсвічування пресета зникає).
- Реалізовано в кожному setter store: метод записує нове значення І чистить `activePresetId`.

### Лічильник на кнопці `Фільтр`
Кожен непустий фільтр додає +1:
- `period` (хоч одне з `dateFrom`/`dateTo` — заповнене) — +1
- `statusFilter.length > 0` — +1
- `counterpartyFilterId` — +1
- `amountMin || amountMax` — +1
- `overdueOnly === true` — +1

Тобто максимум 5. Кнопка показує `Фільтр` коли counter=0, `Фільтр · N` коли counter>0.

### Чипи активних фільтрів
- Рядок з'являється лише коли counter > 0.
- Кожен chip — окремо знімний через `×`. Знімання конкретного фільтра викликає відповідний store-action (`setDateRange(null,null)` тощо), що тягне за собою `activePresetId = null` і перезавантаження списку.

### Панель фільтра (filtersOpen)
- Кнопка `Фільтр` — toggle. При відкритті — копіюємо поточний state у локальні `panelDateFrom`, `panelDateTo`, `panelStatuses`, `panelAmountMin`, `panelAmountMax`, `panelCounterpartyId` (draft).
- Date inputs — `<input type="date">`.
- Date sub-presets `[Сьогодні / Тиждень / Місяць / Квартал / Рік]` всередині панелі — заповнюють `panelDateFrom`/`panelDateTo`. **Не активують topline preset.**
- Status — масив toggle-чіпів (multi).
- Counterparty — наявний select.
- Amount inputs — `inputmode="decimal"`. UI приймає введення з комою або крапкою; перед передачею у DTO рядок нормалізується (trim, замінити кому на крапку, прибрати пробіли) і валідується як `Decimal` у **major units** (грн), щоб збігтися з `total_amount` у БД. Якщо рядок не парситься — inline-помилка `Некоректна сума`. **Не використовуємо** `parseMoneyToMinor` — він би повернув мінор-юніти і ламав би SQL-порівняння.
- Кнопка `Застосувати` — викликає `applyFilters({...draft})` за один прохід; запит в backend — один.
- Кнопка `Скинути` — обнуляє лише локальний draft (не торкається store, поки користувач не натисне `Застосувати`).
- `Esc` всередині панелі — закриває без apply.

### Кнопка `Очистити` (toolbar, біля `Фільтр`)
- Видима лише коли counter > 0.
- Викликає `clearAllFilters()` — повний reset (включно з `activePresetId`).

### Прибраний пошук
- `<input class="documents-list-search" placeholder="Пошук документів">` повністю видаляється з template.
- `onDocumentSearch` хендлер видаляється.
- `state.query` поле та логіка її передавання — видаляються зі store. Виклик `documentsList()` не передає `query`.
- `DocumentsListRequest.query` у DTO **залишається** з `serde(default)` для backward compatibility (Tauri runtime + майбутнє повернення фічі).

## Контракт UI ↔ DTO ↔ DB

### Спільний тип статусу
`frontend/src/lib/types.ts`:
```ts
export type DocumentStatus = "draft" | "issued" | "signed" | "paid" | "delivered";
```
Той самий рядок передається в DTO як `Vec<String>`. У SQL — `WHERE status::text = ANY($N::text[])`. Невалідні для конкретного типу значення (наприклад `paid` для waybill, `delivered` для act/invoice) природно повертають порожньо для відповідної таблиці.

### `DocumentsState` — нові/змінені поля
```ts
interface DocumentsState {
  // ... існуючі (list, editor, ...) ...
  // ВИДАЛЯЄМО: query: string

  dateFrom: string | null;
  dateTo: string | null;
  statusFilter: string[];
  amountMin: string | null;       // decimal string у major units ("1000.00"), кома/крапка нормалізується перед DTO
  amountMax: string | null;
  overdueOnly: boolean;            // true лише при пресеті "Прострочені"
  activePresetId: string | null;   // "all" | "drafts" | "unpaid" | "overdue" | "this-month" | null
}
```

### `documentsList(...)` API
Перехід з позиційних аргументів на single object argument:
```ts
documentsList({
  direction?: DocumentDirection,
  kind?: DocumentKind,
  counterpartyId?: string,
  dateFrom?: string,
  dateTo?: string,
  statuses?: string[],
  amountMin?: string,
  amountMax?: string,
  overdueOnly?: boolean,
}): Promise<DocumentsListDto>
```

### `DocumentsListRequest` (Rust DTO)
```rust
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DocumentsListRequest {
    pub query: Option<String>,
    pub direction: Option<DocumentDirection>,
    pub kind: Option<String>,
    pub counterparty_id: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub statuses: Option<Vec<String>>,
    pub amount_min: Option<Decimal>,
    pub amount_max: Option<Decimal>,
    pub overdue_only: Option<bool>,
}
```

### Setters у store (granular)
- `setDateRange(from, to)` — записує + чистить `activePresetId`.
- `setStatusFilter(statuses)` — те саме.
- `setAmountRange(min, max)` — те саме.
- `setCounterpartyFilter(id)` — те саме.
- `applyPreset(presetId)` — встановлює комбінацію + `activePresetId = presetId`.
- `applyFilters(draft)` — batch-апдейт з панелі (один reload).
- `clearAllFilters()` — повний reset.

Усі викликають `reloadList(snap)` з `filterSeq` race-захистом (як уже в існуючих `setKindFilter`/`setTab`).

## Backend (Rust + SQL)

### `db::*::list_filtered` — нова сигнатура
```rust
pub async fn list_filtered(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<&[String]>,        // зміна: було Option<XxxStatus>
    direction: Option<DocumentDirection>,
    search_query: Option<&str>,
    counterparty_id: Option<Uuid>,
    date_from: Option<NaiveDate>,
    date_to: Option<NaiveDate>,
    amount_min: Option<Decimal>,             // НОВЕ
    amount_max: Option<Decimal>,             // НОВЕ
    overdue_only: bool,                      // НОВЕ
) -> Result<Vec<...>>
```

**Зворотна сумісність із існуючими callers (single-status):** план імплементації перевірить усіх викликачів `list_filtered` у репо. Якщо їх більше одного (наприклад звіти/dashboard), переписуємо їх передавати `Some(&[status.as_str().to_string()])` — це 1-2 рядки на caller. Якщо багато — створюємо thin-wrapper `list_filtered_single_status` для legacy шляху. Рішення фіксує плановий етап.

### SQL фрагменти (alias `a`/`i`/`w`)
```rust
if let Some(statuses) = status_filter.filter(|s| !s.is_empty()) {
    qb.push(" AND ").push(alias).push(".status::text = ANY(");
    qb.push_bind(statuses).push("::text[])");
}
if let Some(from) = date_from {
    qb.push(" AND ").push(alias).push(".date >= ").push_bind(from);
}
if let Some(to) = date_to {
    qb.push(" AND ").push(alias).push(".date <= ").push_bind(to);
}
if let Some(min) = amount_min {
    qb.push(" AND ").push(alias).push(".total_amount >= ").push_bind(min);
}
if let Some(max) = amount_max {
    qb.push(" AND ").push(alias).push(".total_amount <= ").push_bind(max);
}
// overdue_only — лише для acts/invoices (waybill не має expected_payment_date).
// Свідомо виключаємо draft: чернетка не може бути просроченою — вона ще не виставлена клієнту.
// Тому фільтр — позитивний whitelist (issued/signed), а не NOT IN(...).
if overdue_only {
    qb.push(" AND ").push(alias)
      .push(".expected_payment_date IS NOT NULL AND ")
      .push(alias).push(".expected_payment_date < ").push_bind(today)
      .push(" AND ").push(alias).push(".status::text IN ('issued','signed')");
}
```

**Перевірено** (`grep total_amount` у `src/db/`): стовпець `total_amount` має однакову назву у `acts`, `invoices`, `waybills`. Стовпець `date` — теж. Стовпець `expected_payment_date` — лише в `acts`/`invoices`, у `waybills` його немає. SQL пишемо без здогадок.

### `documents_list` (api.rs) — поведінка
```rust
let include_waybills = request.kind.as_deref().map_or(true, |k| k == "waybill")
                       && !overdue_only;   // overdue не має сенсу для waybill
```
Тобто при `overdue_only = true` waybill-гілка `tokio::join!` повертає `Ok(vec![])` без удару в БД. Користувач отримує лише acts+invoices, що семантично коректно.

### Паралельність
`tokio::join!` для трьох таблиць — як зараз (lessons.md правило). Нічого не послідовне.

### `cargo sqlx prepare`
Поточні `list_filtered` побудовані на runtime `QueryBuilder` / `query_as::<_, T>()` — це навмисний вибір (коментар у файлах: "не потребує `cargo sqlx prepare`"). Нові фрагменти продовжують той самий шлях, тому метаданих у `.sqlx` не з'явиться. Прогон `cargo sqlx prepare` залишається **verification step** на випадок якщо в плані з'явиться додатковий `query!`/`query_as!` макрос: якщо diff у `.sqlx/*.json` виник — закомітити; якщо ні — пропустити (`lessons.md` правило стосується саме compile-time макросів).

## Пресети — `frontend/src/lib/config/ui.ts`

```ts
export interface DocumentFilterPreset {
  id: string;
  label: string;
  build(today: Date): {
    dateFrom: string | null;
    dateTo: string | null;
    statusFilter: string[];
    amountMin: string | null;
    amountMax: string | null;
    overdueOnly: boolean;
  };
}

export const DOCUMENT_FILTER_PRESETS: DocumentFilterPreset[] = [
  { id: "all",        label: "Усі",          build: () => empty() },
  { id: "drafts",     label: "Чернетки",     build: () => ({ ...empty(), statusFilter: ["draft"] }) },
  { id: "unpaid",     label: "Неоплачені",   build: () => ({ ...empty(), statusFilter: ["issued","signed"] }) },
  { id: "overdue",    label: "Прострочені",  build: () => ({ ...empty(), overdueOnly: true }) },
  { id: "this-month", label: "Цього місяця", build: (today) => ({ ...empty(), dateFrom: firstOfMonth(today), dateTo: iso(today) }) },
];
```

де `empty()` повертає об'єкт з усіма полями `null` / `[]` / `false`.

`DOCUMENT_STATUS_OPTIONS` (multi-select):
```ts
[
  { value: "draft",     label: "Чернетка" },
  { value: "issued",    label: "Виставлено" },
  { value: "signed",    label: "Підписано" },
  { value: "paid",      label: "Оплачено" },
  { value: "delivered", label: "Доставлено" },
]
```

## Тести

### Frontend — `DocumentsScreen.test.ts`
**Видаляємо:**
- Перевірку `placeholder="Пошук документів"` (тест `routes create, search and editor actions...` — приберемо search-частину; перейменуємо в `routes create and editor actions...`).
- Поле `query` з усіх `mocks.documentsState.set({...})` об'єктів.
- Mock `mocks.load` — більше не викликається з UI.

**Додаємо:**
1. `renders preset chips and applies preset on click` — клік по `[Неоплачені]` викликає `mocks.applyPreset("unpaid")`.
2. `opens filter panel and shows date/status/amount controls` — після `[Фільтр]` панель показує date inputs, status checkboxes, amount inputs, counterparty select.
3. `shows filter counter badge on Filter button` — стан з 3 активними фільтрами → текст кнопки містить `· 3`.
4. `renders active filter chips with × removal` — chip `[Період: ... ×]` клік → `mocks.setDateRange(null, null)`.
5. `Apply дбатчить усі поля одним викликом applyFilters` — змінюємо date+status+amount у панелі, тиснемо `Застосувати` → `applyFilters` викликається 1 раз з повним об'єктом, гранулярні setters НЕ викликаються.
6. `Clear all збиває все` — клік `[Очистити]` → `clearAllFilters`.

### Frontend — `documents.test.ts` (новий store-тест)
Новий файл `frontend/src/lib/stores/__tests__/documents.test.ts`:
1. `applyPreset("unpaid") sets statuses + reloads list once` — мок `appInvoke`, перевіряємо аргументи виклику `documents_list`.
2. `setDateRange(...) clears activePresetId after applyPreset` — після пресета ставимо нову дату → `activePresetId === null`.
3. `setStatusFilter([]) очищує фільтр` — порожній масив = "всі статуси".
4. `applyFilters merges all draft fields and reloads once` — один `appInvoke` виклик з усіма полями.
5. `clearAllFilters resets all filter fields incl. activePresetId і overdueOnly`.
6. Race-protection — швидкі послідовні `applyFilters` дозволяють лише останньому оновити state (як `filterSeq`).

### Backend — `tests/db_integration.rs`
Gated на `TEST_DATABASE_URL`:
1. `list_filtered повертає лише акти у заданому date_range`.
2. `list_filtered з multi-status фільтрує через ANY(...)` — seed acts(draft, issued, paid), фільтр `["draft","paid"]` → 2 рядки.
3. `amount_min/amount_max обмежують total_amount` — seed 3 акти 500/5000/50000, фільтр `min=1000, max=10000` → 1.
4. Аналогічні (1)–(3) для `invoices` і `waybills`.
5. `overdue_only=true для acts повертає лише прострочені невиплачені` — seed 4 акти (paid простр., issued простр., issued майб., draft) → 1 (issued простр.).
6. `overdue_only=true для waybill — короткий цикл повертає 0` — vertical-slice через `documents_list`.

### Backend — `tests/tauri_vertical_slice.rs`
1. `documents_list з period+status+amount+counterparty повертає коректну комбінацію`.

## Edge cases

- `dateFrom > dateTo` — UI блокує `Застосувати`, inline-помилка `Кінцева дата раніше за початкову`.
- `amountMin > amountMax` — те саме, inline `Максимальна сума менша за мінімальну`.
- Невалідне число у amount input — `parseMoneyToMinor` повертає `null` → inline `Некоректна сума`.
- Пресет `Прострочені` + tab `Вхідні` — два рівні фільтрів комбінуються AND. Користувач отримує прострочені вхідні (нагадування про власні борги). Це коректна поведінка.
- Активний `kind = "waybill"` + пресет `Прострочені` — список порожній (waybill не має `expected_payment_date`). Empty state показує існуючий `Поки що документів немає`.
- `kindFilter` і `direction` (tab) — окремі рівні фільтрації, **пресети їх не торкають** (зміна 1 ↔ 4 не повинна несподівано перемикнути tab).

## Файли, які торкаємо

**Frontend:**
- `frontend/src/lib/screens/DocumentsScreen.svelte` — UI (видалити input, додати пресети, активні чіпи, розширена панель, лічильник).
- `frontend/src/lib/stores/documents.ts` — нові поля state, нові setters, видалити `query`/`load(query)` UI шлях.
- `frontend/src/lib/api.ts` — `documentsList` single-object argument.
- `frontend/src/lib/types.ts` — `DocumentStatus`, оновити `DocumentsListRequest`.
- `frontend/src/lib/config/ui.ts` — `DOCUMENT_FILTER_PRESETS`, `DOCUMENT_STATUS_OPTIONS`, `DOCUMENTS_FILTER_COPY`.
- `frontend/src/styles/documents.css` — стилі presets row, active chips, розширена панель grid, лічильник.
- `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts` — оновити (видалити placeholder, додати нові тести).
- `frontend/src/lib/stores/__tests__/documents.test.ts` — новий файл.

**Backend:**
- `src/tauri_api/documents/dto.rs` — нові поля `DocumentsListRequest`.
- `src/tauri_api/documents/api.rs` — прокидування нових параметрів, waybill-skip для overdue.
- `src/db/acts.rs`, `invoices.rs`, `waybills.rs` — `list_filtered` нова сигнатура, нові SQL-блоки.
- `tests/db_integration.rs` — нові тести.
- `tests/tauri_vertical_slice.rs` — vertical slice тест.
- `.sqlx/*.json` — лише якщо план додасть `query!`/`query_as!` макрос (`QueryBuilder`-шлях метаданих не пише).

**Tauri command:**
- `src-tauri/src/commands/documents.rs` — лише прокидування DTO; нічого додавати не треба.

## Ризики та компроміси

1. **Зворотна сумісність `list_filtered` callers.** Зміна сигнатури `status_filter: Option<XxxStatus>` → `Option<&[String]>` — потенційно ламає інших викликачів. План перевірить grep `list_filtered\(` і вирішить: переписати inline (1-2 рядки) або зробити thin-wrapper. Не блокер, але треба пильнувати.
2. **`overdue_only` природно skip-ить waybill.** Користувач очікує що преsset "Прострочені" — це лише про оплату. Цей компроміс зафіксований у backend; UI текст пресета — `Прострочені`, без уточнень. Якщо постане потреба показувати "недоставлені" накладні — це окремий пресет в майбутньому.
3. **Видалений текстовий пошук — потенційний UX-регрес.** Якщо в реальному використанні з'ясується що counterparty-select недостатній (багато контрагентів, треба шукати по номеру документа) — повернемо текстовий пошук всередину панелі окремим тікетом. DTO `query` поле залишене саме для цього.
4. **Date sub-presets в панелі vs topline-пресети — два рівні UX.** Sub-presets всередині панелі заповнюють лише дати і **не активують** topline preset. Може заплутати — план імплементації включить hover-tooltip або subtle separator.

## Послідовність робіт (high-level)

1. Backend: розширити `list_filtered` (3 файли) + DTO; написати backend-тести.
2. Прогнати `cargo sqlx prepare` як verification — закомітити `.sqlx/*.json` лише якщо з'явився diff.
3. Frontend types + api.ts (single-object arg).
4. Frontend store: видалити `query`/`load`-UI, додати нові поля/setters/`applyFilters`.
5. Frontend screen: видалити input пошуку, додати presets row, розширити панель, додати active-chips і лічильник.
6. Стилі.
7. Frontend тести: оновити screen-тести, написати store-тести.
8. Manual smoke в `cargo tauri dev`.
