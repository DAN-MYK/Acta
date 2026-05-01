# BAS Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add two Tauri commands (`import_bas_plan`, `import_bas_execute`) and a Svelte UI that lets users preview and execute BAS data import from the Settings → Integrations screen.

**Architecture:** `src/tauri_api/import.rs` holds DTOs, file-routing logic, and two async functions. `import_bas_plan` scans `storage/import/bas/`, routes files by extension/name keyword, parse-counts xml/xlsx types (no DB), and runs payments dry-run for accurate preview. `import_bas_execute` calls existing `import_*_from_*` functions in order: counterparties → contracts → acts → invoices → payments. The frontend adds a Svelte store, two API calls, and an inline panel in SettingsScreen.

**Tech Stack:** Rust (anyhow, tokio::fs), existing `src/import/bas_*` parsers, Svelte writable store, TypeScript, Tauri commands.

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/tauri_api/import.rs` | Create | DTOs, file routing, `import_bas_plan`, `import_bas_execute` |
| `src/tauri_api/mod.rs` | Modify | Add `pub mod import;` |
| `src-tauri/src/commands/import.rs` | Create | Thin Tauri command wrappers |
| `src-tauri/src/commands/mod.rs` | Modify | Add `pub mod import;` |
| `src-tauri/src/lib.rs` | Modify | Register 2 commands in invoke_handler |
| `frontend/src/lib/types.ts` | Modify | 4 new interfaces |
| `frontend/src/lib/api.ts` | Modify | 2 new API functions + imports |
| `frontend/src/lib/stores/import.ts` | Create | Import store with plan/execute/reset |
| `frontend/src/lib/screens/SettingsScreen.svelte` | Modify | BAS "Імпортувати" button + inline panel |

---

## Existing APIs to call (read-only context)

```
src/import/bas_counterparties.rs:
  parse_counterparties_xml_file(path: &Path) -> Result<Vec<ImportedCounterparty>>
  import_counterparties_from_xml(pool, company_id, path, dry_run) -> Result<CounterpartyImportReport>
  CounterpartyImportReport { parsed, created, updated, skipped, conflicts, rows }

src/import/bas_contracts.rs:
  parse_contracts_xml_file(path: &Path) -> Result<Vec<ImportedContract>>
  import_contracts_from_xml(pool, company_id, path, dry_run) -> Result<ContractImportReport>
  ContractImportReport { parsed, created, updated, skipped, conflicts, rows }

src/import/bas_acts.rs:
  parse_acts_xml_file(path: &Path) -> Result<Vec<ImportedAct>>
  import_acts_from_xml(pool, company_id, path, dry_run) -> Result<ActImportReport>
  ActImportReport { parsed, created, updated, skipped, conflicts, rows }

src/import/bas_invoices.rs:
  parse_invoices_file(path: &Path) -> Result<Vec<ImportedInvoice>>
  import_invoices_from_file(pool, company_id, path, dry_run) -> Result<InvoiceImportReport>
  InvoiceImportReport { parsed, created, updated, skipped, conflicts, rows }

src/import/bas_payments.rs:
  parse_payments_csv_file(path: &Path) -> Result<Vec<ParsedBankRow>>
  apply_imported_payments(pool, company_id, rows, dry_run) -> Result<PaymentImportReport>
  import_payments_from_csv(pool, company_id, path, dry_run) -> Result<PaymentImportReport>
  PaymentImportReport { parsed, created, updated, skipped, conflicts, rows }

AppCtx access: ctx.pool() -> &PgPool, ctx.company_id() -> Uuid
```

---

## Task 1: Rust — DTOs + file routing + import_bas_plan

**Files:**
- Create: `src/tauri_api/import.rs`
- Modify: `src/tauri_api/mod.rs`

- [x] **Step 1: Write the failing tests for `route_file`**

Create `src/tauri_api/import.rs` with tests only (stub the missing items):

```rust
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_csv_is_payments() {
        assert_eq!(route_file(Path::new("bank_export.csv")), Some(FileType::Payments));
        assert_eq!(route_file(Path::new("payments.csv")), Some(FileType::Payments));
    }

    #[test]
    fn route_xml_by_filename_keyword() {
        assert_eq!(route_file(Path::new("counterparties.xml")), Some(FileType::Counterparties));
        assert_eq!(route_file(Path::new("counterpart_2024.xlsx")), Some(FileType::Counterparties));
        assert_eq!(route_file(Path::new("contracts_2024.xml")), Some(FileType::Contracts));
        assert_eq!(route_file(Path::new("acts.xml")), Some(FileType::Acts));
        assert_eq!(route_file(Path::new("invoices.xlsx")), Some(FileType::Invoices));
    }

    #[test]
    fn route_unrecognized_returns_none() {
        assert_eq!(route_file(Path::new("data.txt")), None);
        assert_eq!(route_file(Path::new("report.xml")), None);
    }
}
```

- [x] **Step 2: Run test to verify it fails**

```bash
cargo test --lib tauri_api::import::tests
```

Expected: FAIL — `route_file` and `FileType` not defined

- [x] **Step 3: Replace `src/tauri_api/import.rs` with full implementation**

```rust
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use tokio::fs;

use crate::app_ctx::AppCtx;
use crate::import::bas_acts::parse_acts_xml_file;
use crate::import::bas_contracts::parse_contracts_xml_file;
use crate::import::bas_counterparties::parse_counterparties_xml_file;
use crate::import::bas_invoices::parse_invoices_file;
use crate::import::bas_payments::{apply_imported_payments, parse_payments_csv_file};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportEntityPlanDto {
    pub entity_type: String,
    pub file_name: String,
    pub parsed: usize,
    pub will_create: usize,
    pub will_skip: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileType {
    Counterparties,
    Contracts,
    Acts,
    Invoices,
    Payments,
}

fn route_file(path: &Path) -> Option<FileType> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    let name = path.file_stem()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "csv" => Some(FileType::Payments),
        "xml" | "xlsx" | "xls" => {
            if name.contains("counterpart") || name.contains("контрагент") {
                Some(FileType::Counterparties)
            } else if name.contains("contract")
                || name.contains("договор")
                || name.contains("договір")
            {
                Some(FileType::Contracts)
            } else if (name.contains("act") || name.contains("акт"))
                && !name.contains("contract")
                && !name.contains("договор")
                && !name.contains("договір")
            {
                Some(FileType::Acts)
            } else if name.contains("invoice")
                || name.contains("рахунок")
                || name.contains("накладна")
            {
                Some(FileType::Invoices)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn bas_import_dir() -> PathBuf {
    PathBuf::from("storage/import/bas")
}

async fn collect_sorted_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub async fn import_bas_plan(ctx: &AppCtx) -> Result<ImportPlanDto> {
    let dir = bas_import_dir();
    fs::create_dir_all(&dir).await?;
    let files = collect_sorted_files(&dir).await?;

    const ENTITY_TYPES: &[(&str, FileType)] = &[
        ("counterparties", FileType::Counterparties),
        ("contracts", FileType::Contracts),
        ("acts", FileType::Acts),
        ("invoices", FileType::Invoices),
        ("payments", FileType::Payments),
    ];

    let mut entities = Vec::new();
    for &(entity_type, file_type) in ENTITY_TYPES {
        let matched = files.iter().find(|p| route_file(p) == Some(file_type));
        let dto = match matched {
            None => ImportEntityPlanDto {
                entity_type: entity_type.to_string(),
                file_name: String::new(),
                parsed: 0,
                will_create: 0,
                will_skip: 0,
                error: None,
            },
            Some(path) => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if file_type == FileType::Payments {
                    match parse_payments_csv_file(path).await {
                        Err(e) => ImportEntityPlanDto {
                            entity_type: entity_type.to_string(),
                            file_name,
                            parsed: 0,
                            will_create: 0,
                            will_skip: 0,
                            error: Some(e.to_string()),
                        },
                        Ok(rows) => {
                            match apply_imported_payments(ctx.pool(), ctx.company_id(), &rows, true)
                                .await
                            {
                                Ok(report) => ImportEntityPlanDto {
                                    entity_type: entity_type.to_string(),
                                    file_name,
                                    parsed: report.parsed,
                                    will_create: report.created,
                                    will_skip: report.skipped,
                                    error: None,
                                },
                                Err(e) => ImportEntityPlanDto {
                                    entity_type: entity_type.to_string(),
                                    file_name,
                                    parsed: rows.len(),
                                    will_create: 0,
                                    will_skip: 0,
                                    error: Some(e.to_string()),
                                },
                            }
                        }
                    }
                } else {
                    let count_result: Result<usize> = match file_type {
                        FileType::Counterparties => {
                            parse_counterparties_xml_file(path).await.map(|r| r.len())
                        }
                        FileType::Contracts => {
                            parse_contracts_xml_file(path).await.map(|r| r.len())
                        }
                        FileType::Acts => parse_acts_xml_file(path).await.map(|r| r.len()),
                        FileType::Invoices => parse_invoices_file(path).await.map(|r| r.len()),
                        FileType::Payments => unreachable!(),
                    };
                    match count_result {
                        Ok(parsed) => ImportEntityPlanDto {
                            entity_type: entity_type.to_string(),
                            file_name,
                            parsed,
                            will_create: 0,
                            will_skip: 0,
                            error: None,
                        },
                        Err(e) => ImportEntityPlanDto {
                            entity_type: entity_type.to_string(),
                            file_name,
                            parsed: 0,
                            will_create: 0,
                            will_skip: 0,
                            error: Some(e.to_string()),
                        },
                    }
                }
            }
        };
        entities.push(dto);
    }

    Ok(ImportPlanDto { entities })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_csv_is_payments() {
        assert_eq!(route_file(Path::new("bank_export.csv")), Some(FileType::Payments));
        assert_eq!(route_file(Path::new("payments.csv")), Some(FileType::Payments));
    }

    #[test]
    fn route_xml_by_filename_keyword() {
        assert_eq!(
            route_file(Path::new("counterparties.xml")),
            Some(FileType::Counterparties)
        );
        assert_eq!(
            route_file(Path::new("counterpart_2024.xlsx")),
            Some(FileType::Counterparties)
        );
        assert_eq!(
            route_file(Path::new("contracts_2024.xml")),
            Some(FileType::Contracts)
        );
        assert_eq!(route_file(Path::new("acts.xml")), Some(FileType::Acts));
        assert_eq!(route_file(Path::new("invoices.xlsx")), Some(FileType::Invoices));
    }

    #[test]
    fn route_unrecognized_returns_none() {
        assert_eq!(route_file(Path::new("data.txt")), None);
        assert_eq!(route_file(Path::new("report.xml")), None);
    }
}
```

- [x] **Step 4: Add `pub mod import;` to `src/tauri_api/mod.rs`**

Append to the file:

```
pub mod import;
```

- [x] **Step 5: Run tests**

```bash
cargo test --lib tauri_api::import::tests
```

Expected: 3 tests pass

- [x] **Step 6: Verify lib compiles**

```bash
cargo build --lib
```

Expected: `Finished` with no errors

- [x] **Step 7: Commit**

```bash
git add src/tauri_api/import.rs src/tauri_api/mod.rs
git commit -m "feat: add import_bas_plan with DTOs and file routing"
```

---

## Task 2: Rust — add import_bas_execute

**Files:**
- Modify: `src/tauri_api/import.rs`

- [x] **Step 1: Add execute-specific imports at the top of `src/tauri_api/import.rs`**

Append to the existing use block (after the plan imports):

```rust
use crate::import::bas_acts::import_acts_from_xml;
use crate::import::bas_contracts::import_contracts_from_xml;
use crate::import::bas_counterparties::import_counterparties_from_xml;
use crate::import::bas_invoices::import_invoices_from_file;
use crate::import::bas_payments::import_payments_from_csv;
```

- [x] **Step 2: Append `import_bas_execute` to `src/tauri_api/import.rs` (before the `#[cfg(test)]` block)**

```rust
pub async fn import_bas_execute(ctx: &AppCtx) -> Result<ImportResultDto> {
    let dir = bas_import_dir();
    fs::create_dir_all(&dir).await?;
    let files = collect_sorted_files(&dir).await?;

    const ENTITY_TYPES: &[(&str, FileType)] = &[
        ("counterparties", FileType::Counterparties),
        ("contracts", FileType::Contracts),
        ("acts", FileType::Acts),
        ("invoices", FileType::Invoices),
        ("payments", FileType::Payments),
    ];

    let mut entities = Vec::new();
    for &(entity_type, file_type) in ENTITY_TYPES {
        let matched = files.iter().find(|p| route_file(p) == Some(file_type));
        let dto = match matched {
            None => ImportEntityResultDto {
                entity_type: entity_type.to_string(),
                created: 0,
                updated: 0,
                skipped: 0,
                conflicts: 0,
                error: None,
            },
            Some(path) => {
                let pool = ctx.pool();
                let company_id = ctx.company_id();
                let result = match file_type {
                    FileType::Counterparties => {
                        import_counterparties_from_xml(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                    FileType::Contracts => {
                        import_contracts_from_xml(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                    FileType::Acts => {
                        import_acts_from_xml(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                    FileType::Invoices => {
                        import_invoices_from_file(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                    FileType::Payments => {
                        import_payments_from_csv(pool, company_id, path, false)
                            .await
                            .map(|r| (r.created, r.updated, r.skipped, r.conflicts))
                    }
                };
                match result {
                    Ok((created, updated, skipped, conflicts)) => ImportEntityResultDto {
                        entity_type: entity_type.to_string(),
                        created,
                        updated,
                        skipped,
                        conflicts,
                        error: None,
                    },
                    Err(e) => ImportEntityResultDto {
                        entity_type: entity_type.to_string(),
                        created: 0,
                        updated: 0,
                        skipped: 0,
                        conflicts: 0,
                        error: Some(e.to_string()),
                    },
                }
            }
        };
        entities.push(dto);
    }

    Ok(ImportResultDto { entities })
}
```

- [x] **Step 3: Build to verify compilation**

```bash
cargo build --lib
```

Expected: `Finished` with no errors

- [x] **Step 4: Commit**

```bash
git add src/tauri_api/import.rs
git commit -m "feat: add import_bas_execute"
```

---

## Task 3: Wire Tauri commands

**Files:**
- Create: `src-tauri/src/commands/import.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Create `src-tauri/src/commands/import.rs`**

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

- [x] **Step 2: Add `pub mod import;` to `src-tauri/src/commands/mod.rs`**

Append to the file:

```
pub mod import;
```

- [x] **Step 3: Register commands in `src-tauri/src/lib.rs`**

In the `tauri::generate_handler![...]` block, add after `commands::payments::payment_unreconcile`:

```rust
            commands::import::import_bas_plan,
            commands::import::import_bas_execute,
```

- [x] **Step 4: Build Tauri binary**

```bash
cd src-tauri && cargo build
```

Expected: `Finished` with no errors

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/commands/import.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: register import_bas_plan and import_bas_execute Tauri commands"
```

---

## Task 4: TypeScript types + API

**Files:**
- Modify: `frontend/src/lib/types.ts`
- Modify: `frontend/src/lib/api.ts`

- [x] **Step 1: Append 4 interfaces to `frontend/src/lib/types.ts`**

Add at the end of the file:

```typescript
export interface ImportEntityPlanDto {
  entityType: string;
  fileName: string;
  parsed: number;
  willCreate: number;
  willSkip: number;
  error: string | null;
}

export interface ImportPlanDto {
  entities: ImportEntityPlanDto[];
}

export interface ImportEntityResultDto {
  entityType: string;
  created: number;
  updated: number;
  skipped: number;
  conflicts: number;
  error: string | null;
}

export interface ImportResultDto {
  entities: ImportEntityResultDto[];
}
```

- [x] **Step 2: Add imports + 2 API functions to `frontend/src/lib/api.ts`**

In the existing type import block at the top of `api.ts`, add `ImportPlanDto` and `ImportResultDto` to the list.

Then append at the end of the file:

```typescript
export const importBasPlan = () => invoke<ImportPlanDto>("import_bas_plan");
export const importBasExecute = () => invoke<ImportResultDto>("import_bas_execute");
```

- [x] **Step 3: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit
```

Expected: no errors

- [x] **Step 4: Commit**

```bash
git add frontend/src/lib/types.ts frontend/src/lib/api.ts
git commit -m "feat: add BAS import TypeScript types and API functions"
```

---

## Task 5: Svelte import store

**Files:**
- Create: `frontend/src/lib/stores/import.ts`

- [x] **Step 1: Create `frontend/src/lib/stores/import.ts`**

```typescript
import { writable } from "svelte/store";
import { importBasPlan, importBasExecute } from "../api";
import type { ImportPlanDto, ImportResultDto } from "../types";

interface ImportState {
  plan: ImportPlanDto | null;
  result: ImportResultDto | null;
  loading: boolean;
  error: string | null;
}

function createImportStore() {
  const { subscribe, update, set } = writable<ImportState>({
    plan: null,
    result: null,
    loading: false,
    error: null
  });

  async function plan() {
    update((state) => ({ ...state, loading: true, error: null }));
    try {
      const data = await importBasPlan();
      update((state) => ({ ...state, plan: data, loading: false }));
    } catch (error) {
      update((state) => ({ ...state, loading: false, error: String(error) }));
    }
  }

  async function execute() {
    update((state) => ({ ...state, loading: true, error: null }));
    try {
      const data = await importBasExecute();
      update((state) => ({ ...state, result: data, loading: false }));
    } catch (error) {
      update((state) => ({ ...state, loading: false, error: String(error) }));
    }
  }

  function reset() {
    set({ plan: null, result: null, loading: false, error: null });
  }

  return { subscribe, plan, execute, reset };
}

export const importStore = createImportStore();
```

- [x] **Step 2: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit
```

Expected: no errors

- [x] **Step 3: Commit**

```bash
git add frontend/src/lib/stores/import.ts
git commit -m "feat: add Svelte import store"
```

---

## Task 6: SettingsScreen — BAS import button + inline panel

**Files:**
- Modify: `frontend/src/lib/screens/SettingsScreen.svelte`

- [x] **Step 1: Add import store import to the `<script>` block**

After the existing imports at the top of the `<script lang="ts">` block, add:

```typescript
  import { importStore } from "../stores/import";

  const importBas = importStore;
  let showBasImport = false;
```

- [x] **Step 2: Replace the integrations section with the version that includes the BAS button and inline panel**

Find this block in the template:

```svelte
      {:else if $settings.section === "integrations"}
        <div class="settings-card">
          <h3 class="title-with-icon"><AppIcon name="integrations" surface={true} size={18} /><span>Інтеграції</span></h3>
          <div class="linked-list">
            {#each $settings.screen?.integrations ?? [] as integration}
              <div class="settings-row">
                <div>
                  <strong>{integration.label}</strong>
                  <p>{integration.description}</p>
                </div>
                <div class="settings-row-actions">
                  <span>{integration.enabled ? "Активно" : "Вимкнено"}</span>
                  <button class="action-button compact" on:click={() => settings.configureIntegration(integration.tag)}>
                    <AppIcon name={integration.enabled ? "edit" : "add"} size={14} />
                    <span>{integration.enabled ? "Налаштувати" : "Підключити"}</span>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>
```

Replace it with:

```svelte
      {:else if $settings.section === "integrations"}
        <div class="settings-card">
          <h3 class="title-with-icon"><AppIcon name="integrations" surface={true} size={18} /><span>Інтеграції</span></h3>
          <div class="linked-list">
            {#each $settings.screen?.integrations ?? [] as integration}
              <div class="settings-row">
                <div>
                  <strong>{integration.label}</strong>
                  <p>{integration.description}</p>
                </div>
                <div class="settings-row-actions">
                  <span>{integration.enabled ? "Активно" : "Вимкнено"}</span>
                  <button class="action-button compact" on:click={() => settings.configureIntegration(integration.tag)}>
                    <AppIcon name={integration.enabled ? "edit" : "add"} size={14} />
                    <span>{integration.enabled ? "Налаштувати" : "Підключити"}</span>
                  </button>
                  {#if integration.tag === "bas"}
                    <button
                      class="action-button compact"
                      on:click={() => { showBasImport = !showBasImport; if (!showBasImport) importBas.reset(); }}
                    >
                      <AppIcon name="import" size={14} />
                      <span>Імпортувати</span>
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>

          {#if showBasImport}
            <div class="settings-card" style="margin-top: 1rem;">
              {#if $importBas.error}
                <p class="error">{$importBas.error}</p>
              {/if}

              {#if $importBas.result === null}
                <p>Помістіть файли BAS у <code>storage/import/bas/</code></p>
                <div class="settings-actions-row" style="margin-top: 0.5rem;">
                  <button
                    class="action-button compact"
                    on:click={() => importBas.plan()}
                    disabled={$importBas.loading}
                  >
                    <AppIcon name="refresh" size={14} />
                    <span>{$importBas.loading ? "Перевірка..." : "Перевірити файли"}</span>
                  </button>
                </div>

                {#if $importBas.plan !== null}
                  <div class="reports-table" style="margin-top: 1rem;">
                    <div class="reports-table-row reports-table-head reports-table-wide">
                      <span>Тип</span><span>Файл</span><span>Записів</span><span>Новий / Дублікат</span>
                    </div>
                    {#each $importBas.plan.entities as entity}
                      <div class="reports-table-row reports-table-wide" class:error={!!entity.error}>
                        <span>{entity.entityType}</span>
                        <span>{entity.fileName || "—"}</span>
                        <span>{entity.parsed || "—"}</span>
                        <span>
                          {#if entity.error}
                            {entity.error}
                          {:else if entity.entityType === "payments" && entity.fileName}
                            {entity.willCreate} нових / {entity.willSkip} дублікатів
                          {:else}
                            —
                          {/if}
                        </span>
                      </div>
                    {/each}
                  </div>
                  <div class="settings-actions-row" style="margin-top: 0.5rem;">
                    <button
                      class="action-button compact"
                      on:click={() => importBas.execute()}
                      disabled={$importBas.loading}
                    >
                      <AppIcon name="save" size={14} />
                      <span>{$importBas.loading ? "Виконання..." : "Виконати імпорт"}</span>
                    </button>
                    <button
                      class="action-button compact"
                      on:click={() => { showBasImport = false; importBas.reset(); }}
                    >
                      <span>Скасувати</span>
                    </button>
                  </div>
                {/if}
              {:else}
                <div class="reports-table">
                  <div class="reports-table-row reports-table-head reports-table-wide">
                    <span>Тип</span><span>Створено</span><span>Оновлено</span><span>Пропущено</span><span>Конфлікти</span>
                  </div>
                  {#each $importBas.result.entities as entity}
                    <div class="reports-table-row reports-table-wide" class:error={!!entity.error}>
                      <span>{entity.entityType}</span>
                      <span>{entity.created}</span>
                      <span>{entity.updated}</span>
                      <span>{entity.skipped}</span>
                      <span>{entity.conflicts}</span>
                    </div>
                  {/each}
                </div>
                {#each $importBas.result.entities.filter((e) => e.error) as entity}
                  <p class="error">{entity.entityType}: {entity.error}</p>
                {/each}
                <div class="settings-actions-row" style="margin-top: 0.5rem;">
                  <button
                    class="action-button compact"
                    on:click={() => { showBasImport = false; importBas.reset(); }}
                  >
                    <span>Закрити</span>
                  </button>
                </div>
              {/if}
            </div>
          {/if}
        </div>
```

- [x] **Step 3: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit
```

Expected: no errors

- [x] **Step 4: Test manually in dev mode**

Start Tauri dev mode from `src-tauri/`:
```bash
cargo tauri dev
```

Navigate to Settings → Інтеграції. Verify:
1. BAS row has an "Імпортувати" button
2. Clicking it toggles the inline panel
3. "Перевірити файли" button calls `import_bas_plan` (creates `storage/import/bas/` if missing)
4. After plan returns, the entity table shows with file names and record counts
5. "Виконати імпорт" calls `import_bas_execute` and shows the results table
6. "Скасувати" / "Закрити" hides the panel and resets state

- [x] **Step 5: Commit**

```bash
git add frontend/src/lib/screens/SettingsScreen.svelte
git commit -m "feat: add BAS import button and inline panel to SettingsScreen"
```
