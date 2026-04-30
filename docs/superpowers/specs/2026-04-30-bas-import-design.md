# BAS Import — Design Spec

Дата: 2026-04-30

## Мета

Дати користувачу можливість імпортувати дані з BAS (1С:Підприємство) у Acta через UI.
Підтримувані типи: контрагенти, договори, акти, накладні, платежі (банківські виписки з BAS).

---

## Data Flow

```
storage/import/bas/          ← юзер кладе файли сюди
  counterparties.xml
  contracts.xml
  acts.xml
  invoices.xml / invoices.xlsx
  payments.csv               ← банківська виписка з BAS (CSV формат)

[Кнопка "Перевірити файли"]
  → import_bas_plan [Tauri command]
      → сканує storage/import/bas/
      → маршрутизує файли за типом
      → парсить (без DB для xml/xlsx типів)
      → для payments: apply_imported_payments(dry_run=true) — точний preview
      → повертає ImportPlanDto

Frontend показує таблицю: тип / файл / кількість записів / errors

[Кнопка "Виконати імпорт"]
  → import_bas_execute [Tauri command]
      → порядок: counterparties → contracts → acts → invoices → payments
      → кожен: import_*_from_file(pool, company_id, path, dry_run=false)
      → повертає ImportResultDto з реальними числами
```

---

## Маршрутизація файлів

| Розширення | Умова | Парсер |
|------------|-------|--------|
| `.csv` | будь-яка назва | `bas_payments` |
| `.xml` / `.xlsx` | назва містить "counterpart" або "контрагент" | `bas_counterparties` |
| `.xml` | назва містить "contract" або "договор" | `bas_contracts` |
| `.xml` | назва містить "act" або "акт" (але НЕ "contract") | `bas_acts` |
| `.xml` / `.xlsx` | назва містить "invoice", "рахунок" або "накладна" | `bas_invoices` |

Перевірка назви — case-insensitive. Якщо файл не підпадає під жоден шаблон — ігнорується.
Якщо для одного типу знайдено кілька файлів — береться перший за алфавітом.
Якщо `storage/import/bas/` не існує — `import_bas_plan` створює директорію і повертає план з `parsed: 0` для всіх типів (аналогічно до `ensure_manual_import_template` у payments).

**Чому CSV = payments**: тільки платежі експортуються BAS у CSV; решта — XML або XLSX.

---

## Порядок execute

```
counterparties → contracts → acts → invoices → payments
```

Обов'язковий порядок для перших чотирьох: contracts залежать від counterparties (resolve_counterparty_id), acts і invoices залежать від counterparties і contracts. Payments незалежні, запускаються останніми.

**Чому plan НЕ використовує dry_run для xml/xlsx типів**: apply_imported_contracts / apply_imported_acts / apply_imported_invoices під час dry_run звертаються до БД за counterparties. До execute counterparties ще не в БД — plan покаже хибні "skipped". Тому plan лише парсить файли і рахує записи. Точні числа — тільки у результатах execute.

**Payments — виняток**: apply_imported_payments перевіряє дублікати за bank_ref/description, без залежності від counterparties. Dry_run для payments дає точний preview.

---

## Rust — структури даних

```rust
// src/tauri_api/import.rs

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportEntityPlanDto {
    pub entity_type: String,   // "counterparties" | "contracts" | "acts" | "invoices" | "payments"
    pub file_name: String,     // знайдений файл або ""
    pub parsed: usize,
    pub will_create: usize,    // точно тільки для payments; 0 для xml/xlsx типів
    pub will_skip: usize,      // точно тільки для payments; 0 для xml/xlsx типів
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlanDto {
    pub entities: Vec<ImportEntityPlanDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportEntityResultDto {
    pub entity_type: String,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultDto {
    pub entities: Vec<ImportEntityResultDto>,
}

pub async fn import_bas_plan(ctx: &AppCtx) -> Result<ImportPlanDto>
pub async fn import_bas_execute(ctx: &AppCtx) -> Result<ImportResultDto>
```

---

## Rust — файли

| Файл | Зміна |
|------|-------|
| `src/tauri_api/import.rs` | Новий — DTOs + import_bas_plan + import_bas_execute |
| `src/tauri_api/mod.rs` | Додати `pub mod import;` |
| `src-tauri/src/commands/import.rs` | Новий — тонкі обгортки за паттерном проєкту |
| `src-tauri/src/commands/mod.rs` | Додати `pub mod import;` |
| `src-tauri/src/lib.rs` | Додати 2 команди до invoke_handler |

### Паттерн команди (з `commands/import.rs`)

```rust
use acta::tauri_api::import::{ImportPlanDto, ImportResultDto};
use tauri::State;
use crate::TauriState;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn import_bas_plan(state: State<'_, TauriState>) -> CommandResult<ImportPlanDto> {
    acta::tauri_api::import::import_bas_plan(&state.ctx)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_bas_execute(state: State<'_, TauriState>) -> CommandResult<ImportResultDto> {
    acta::tauri_api::import::import_bas_execute(&state.ctx)
        .await
        .map_err(|e| e.to_string())
}
```

---

## TypeScript — типи і API

```typescript
// frontend/src/lib/types.ts — додати
export interface ImportEntityPlanDto {
  entityType: string;
  fileName: string;
  parsed: number;
  willCreate: number;
  willSkip: number;
  error: string | null;
}
export interface ImportPlanDto { entities: ImportEntityPlanDto[] }

export interface ImportEntityResultDto {
  entityType: string;
  created: number;
  updated: number;
  skipped: number;
  conflicts: number;
  error: string | null;
}
export interface ImportResultDto { entities: ImportEntityResultDto[] }

// frontend/src/lib/api.ts — додати
export const importBasPlan    = () => invoke<ImportPlanDto>("import_bas_plan");
export const importBasExecute = () => invoke<ImportResultDto>("import_bas_execute");
```

---

## TypeScript — Import Store

```typescript
// frontend/src/lib/stores/import.ts

interface ImportState {
  plan: ImportPlanDto | null;
  result: ImportResultDto | null;
  loading: boolean;
  error: string | null;
}

// методи:
// plan()    — викликає importBasPlan, зберігає результат
// execute() — викликає importBasExecute, зберігає результат
// reset()   — скидає стан
```

---

## UI — SettingsScreen (секція Інтеграції)

BAS рядок отримує кнопку "Імпортувати" поряд з існуючою "Підключити/Налаштувати".
Натискання відкриває inline-панель нижче списку інтеграцій:

```
Помістіть файли BAS у storage/import/bas/
[ Перевірити файли ]

── після plan (loading=false, plan!=null) ──
Контрагенти  counterparties.xml   50 записів
Договори     contracts.xml        30 записів
Акти         acts.xml            120 записів
Накладні     —                     не знайдено
Платежі      payments.csv    45 → 40 нових / 5 дублікатів

[ Виконати імпорт ]  [ Скасувати ]

── після execute (result!=null) ──
Контрагенти  48 створено / 2 оновлено / 0 конфліктів
Договори     25 створено / 5 пропущено
Акти        115 створено / 5 пропущено
Платежі      40 створено / 5 пропущено
[ Закрити ]
```

Панель показується тільки якщо `$import.plan !== null || $import.result !== null`.
"Скасувати" / "Закрити" → `import.reset()`.

### Файли Frontend

| Файл | Зміна |
|------|-------|
| `frontend/src/lib/types.ts` | Додати 4 інтерфейси |
| `frontend/src/lib/api.ts` | Додати 2 функції |
| `frontend/src/lib/stores/import.ts` | Новий стор |
| `frontend/src/lib/screens/SettingsScreen.svelte` | Кнопка + inline панель для BAS |

---

## Що НЕ входить у scope

- Per-record conflict resolution (existing infrastructure не підтримує)
- File picker dialog (`tauri-plugin-dialog` відсутній у Cargo.toml)
- Автоматичне зведення платежів з документами після імпорту
- Прогрес-бар під час виконання (streaming events)
- Імпорт з підпапок (тільки прямі файли в `storage/import/bas/`)
