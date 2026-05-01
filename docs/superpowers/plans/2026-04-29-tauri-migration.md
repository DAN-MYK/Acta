# Tauri + Svelte + TypeScript Migration Plan

> **Archived/pre-cutover:** execution plan збережено як історія Tauri cutover. Після `2026-04-30` не використовуй його Slint mapping або deletion checklist як live backlog; актуальний стан у `docs/architecture/tauri-runtime.md` і `docs/architecture/tauri-command-surface.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Замінити Slint UI на Tauri + Svelte + TypeScript, зберігши весь Rust бекенд (db/, models/, import/, pdf/).

**Architecture:** Rust бекенд перетворюється на Tauri додаток де кожна Slint-callback стає `#[tauri::command]`. Svelte фронтенд через `@tauri-apps/api` викликає ці команди і зберігає стан у Svelte stores. Дані форматуються на Rust стороні (гроші рядками, дати рядками) — контракт залишається незмінним.

**Tech Stack:** Tauri v2, Svelte 5, TypeScript, `@tauri-apps/api`, CSS custom properties для дизайн-системи, PostgreSQL + sqlx (без змін), serde_json для серіалізації

---

## Структура файлів після міграції

```
acta/
├── src-tauri/               ← весь Rust (було: src/ + ui/)
│   ├── src/
│   │   ├── main.rs          ← Tauri entry point (новий)
│   │   ├── lib.rs           ← реєстрація команд (новий)
│   │   ├── state.rs         ← AppState для Tauri (новий)
│   │   ├── commands/        ← було src/ui/*.rs (перетворені)
│   │   │   ├── mod.rs
│   │   │   ├── dashboard.rs
│   │   │   ├── documents.rs
│   │   │   ├── counterparties.rs
│   │   │   ├── payments.rs
│   │   │   ├── tasks.rs
│   │   │   ├── reports.rs
│   │   │   └── settings.rs
│   │   ├── db/              ← без змін
│   │   ├── models/          ← без змін
│   │   ├── import/          ← без змін
│   │   └── pdf/             ← без змін
│   ├── Cargo.toml           ← новий (Tauri замість Slint)
│   ├── build.rs             ← Tauri build (новий)
│   └── tauri.conf.json      ← конфіг Tauri (новий)
├── src/                     ← Svelte фронтенд (новий)
│   ├── app.svelte
│   ├── lib/
│   │   ├── types.ts         ← TypeScript інтерфейси (з types.slint)
│   │   ├── api.ts           ← invoke() обгортки
│   │   ├── stores/
│   │   │   ├── navigation.ts
│   │   │   ├── documents.ts
│   │   │   ├── counterparties.ts
│   │   │   ├── payments.ts
│   │   │   ├── tasks.ts
│   │   │   └── settings.ts
│   │   ├── components/
│   │   │   ├── Shell.svelte
│   │   │   ├── Sidebar.svelte
│   │   │   ├── Badge.svelte
│   │   │   ├── Button.svelte
│   │   │   ├── SearchInput.svelte
│   │   │   ├── DataTable.svelte
│   │   │   ├── Modal.svelte
│   │   │   ├── KpiCard.svelte
│   │   │   └── BarChart.svelte
│   │   └── styles/
│   │       ├── tokens.css   ← design tokens (з design-tokens.slint)
│   │       └── global.css
│   └── screens/
│       ├── Dashboard.svelte
│       ├── Documents.svelte
│       ├── Counterparties.svelte
│       ├── Payments.svelte
│       ├── Tasks.svelte
│       ├── Reports.svelte
│       └── Settings.svelte
├── migrations/              ← без змін
├── package.json             ← новий
├── vite.config.ts           ← новий
└── svelte.config.js         ← новий
```

---

## Task 1: Scaffold Tauri Project Structure

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `svelte.config.js`
- Create: `index.html`

- [x] **Step 1: Встановити Tauri CLI та залежності**

```bash
npm install --save-dev @tauri-apps/cli @tauri-apps/api
npm install --save-dev vite @sveltejs/vite-plugin-svelte svelte svelte-check typescript
```

- [x] **Step 2: Створити `package.json`**

```json
{
  "name": "acta",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "tauri dev",
    "build": "tauri build",
    "frontend:dev": "vite",
    "frontend:build": "vite build",
    "check": "svelte-check --tsconfig ./tsconfig.json"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "@tauri-apps/cli": "^2.0.0",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^6.0.0"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0"
  }
}
```

- [x] **Step 3: Створити `vite.config.ts`**

```typescript
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
});
```

- [x] **Step 4: Створити `svelte.config.js`**

```javascript
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
};
```

- [x] **Step 5: Створити `tsconfig.json`**

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "allowJs": true,
    "checkJs": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "sourceMap": true,
    "strict": true,
    "target": "ESNext",
    "useDefineForClassFields": true
  }
}
```

- [x] **Step 6: Створити `index.html`**

```html
<!DOCTYPE html>
<html lang="uk">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Acta</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [x] **Step 7: Створити `src/main.ts`**

```typescript
import App from "./app.svelte";

const app = new App({ target: document.getElementById("app")! });

export default app;
```

- [x] **Step 8: Перемістити Rust код в `src-tauri/`**

```bash
mkdir -p src-tauri/src
cp -r src/* src-tauri/src/
cp Cargo.toml src-tauri/Cargo.toml
# НЕ видаляти src/ ще — видалимо в Task 14
```

- [x] **Step 9: Створити `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [x] **Step 10: Створити `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Acta",
  "version": "0.1.0",
  "identifier": "ua.acta.app",
  "build": {
    "beforeDevCommand": "npm run frontend:dev",
    "beforeBuildCommand": "npm run frontend:build",
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420"
  },
  "app": {
    "windows": [
      {
        "title": "Acta",
        "width": 1280,
        "height": 800,
        "minWidth": 1024,
        "minHeight": 640
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png"]
  }
}
```

- [x] **Step 11: Commit**

```bash
git add package.json vite.config.ts svelte.config.js tsconfig.json index.html src/ src-tauri/
git commit -m "chore: scaffold Tauri + Svelte + TypeScript project structure"
```

---

## Task 2: Rust Backend — Новий Cargo.toml та AppState

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/state.rs`
- Create: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/main.rs`

- [x] **Step 1: Оновити `src-tauri/Cargo.toml` — замінити Slint на Tauri**

```toml
[package]
name = "acta"
version = "0.1.0"
edition = "2021"

[lib]
name = "acta_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# DB (без змін)
sqlx = { version = "0.8", features = [
    "runtime-tokio", "postgres", "uuid", "chrono", "rust_decimal", "macros"
] }
tokio = { version = "1", features = ["full"] }
rust_decimal = { version = "1", features = ["serde"] }
rust_decimal_macros = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
anyhow = "1"
thiserror = "2"
dotenvy = "0.15"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Import/PDF (без змін)
quick-xml = { version = "0.37", features = ["serialize"] }
calamine = "0.26"
csv = "1"
lopdf = "0.34"
```

- [x] **Step 2: Створити `src-tauri/src/state.rs`**

```rust
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct AppState {
    pub pool: PgPool,
    pub active_company_id: Arc<Mutex<Uuid>>,
}

impl AppState {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        sqlx::migrate!("../migrations").run(&pool).await?;

        let company_id = sqlx::query_scalar!(
            "SELECT id FROM companies WHERE archived_at IS NULL ORDER BY created_at LIMIT 1"
        )
        .fetch_one(&pool)
        .await?;

        Ok(Self {
            pool,
            active_company_id: Arc::new(Mutex::new(company_id)),
        })
    }

    pub async fn company_id(&self) -> Uuid {
        *self.active_company_id.lock().await
    }
}
```

- [x] **Step 3: Створити `src-tauri/src/lib.rs` (порожній реєстр команд поки)**

```rust
pub mod commands;
pub mod db;
pub mod models;
pub mod import;
pub mod pdf;
pub mod state;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let database_url = std::env::var("DATABASE_URL")
                    .expect("DATABASE_URL must be set");
                let state = state::AppState::new(&database_url)
                    .await
                    .expect("Failed to initialize AppState");
                handle.manage(state);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::save_company,
            commands::dashboard::get_dashboard,
            commands::counterparties::list_counterparties,
            commands::counterparties::get_counterparty_detail,
            commands::counterparties::save_counterparty,
            commands::counterparties::delete_counterparty,
            commands::documents::list_documents,
            commands::documents::get_document,
            commands::documents::save_document,
            commands::documents::change_document_status,
            commands::documents::delete_document,
            commands::payments::list_payments,
            commands::payments::import_payments_csv,
            commands::tasks::list_tasks,
            commands::tasks::save_task,
            commands::tasks::set_task_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [x] **Step 4: Оновити `src-tauri/src/main.rs`**

```rust
fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();
    acta_lib::run();
}
```

- [x] **Step 5: Перевірити компіляцію (без команд ще)**

```bash
cd src-tauri && cargo check 2>&1 | head -50
```

Очікується: помилки про відсутні модулі `commands` — нормально, створимо в Task 3.

- [x] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/state.rs src-tauri/src/lib.rs src-tauri/src/main.rs src-tauri/build.rs
git commit -m "feat: add Tauri backend scaffold with AppState and DB initialization"
```

---

## Task 3: Tauri Commands — Settings та базовий шаблон

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/settings.rs`

Цей task встановлює паттерн для всіх наступних команд.

- [x] **Step 1: Створити `src-tauri/src/commands/mod.rs`**

```rust
pub mod settings;
pub mod dashboard;
pub mod counterparties;
pub mod documents;
pub mod payments;
pub mod tasks;
pub mod reports;
```

- [x] **Step 2: Створити `src-tauri/src/commands/settings.rs`**

```rust
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct SettingsResponse {
    pub company: CompanyInfo,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CompanyInfo {
    pub id: String,
    pub full_name: String,
    pub short_name: String,
    pub edrpou: String,
    pub ipn: String,
    pub address: String,
    pub director: String,
    pub iban: String,
    pub bank: String,
    pub vat_registered: bool,
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsResponse, String> {
    let company_id = state.company_id().await;
    let row = sqlx::query!(
        "SELECT id, full_name, short_name, edrpou, ipn, address, director, iban, bank, vat_registered
         FROM companies WHERE id = $1",
        company_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(SettingsResponse {
        company: CompanyInfo {
            id: row.id.to_string(),
            full_name: row.full_name,
            short_name: row.short_name.unwrap_or_default(),
            edrpou: row.edrpou.unwrap_or_default(),
            ipn: row.ipn.unwrap_or_default(),
            address: row.address.unwrap_or_default(),
            director: row.director.unwrap_or_default(),
            iban: row.iban.unwrap_or_default(),
            bank: row.bank.unwrap_or_default(),
            vat_registered: row.vat_registered.unwrap_or(false),
        },
    })
}

#[derive(Deserialize)]
pub struct SaveCompanyPayload {
    pub full_name: String,
    pub short_name: String,
    pub edrpou: String,
    pub ipn: String,
    pub address: String,
    pub director: String,
    pub iban: String,
    pub bank: String,
    pub vat_registered: bool,
}

#[tauri::command]
pub async fn save_company(
    state: State<'_, AppState>,
    payload: SaveCompanyPayload,
) -> Result<(), String> {
    let company_id = state.company_id().await;
    sqlx::query!(
        "UPDATE companies SET full_name=$2, short_name=$3, edrpou=$4, ipn=$5,
         address=$6, director=$7, iban=$8, bank=$9, vat_registered=$10, updated_at=NOW()
         WHERE id=$1",
        company_id,
        payload.full_name,
        payload.short_name,
        payload.edrpou,
        payload.ipn,
        payload.address,
        payload.director,
        payload.iban,
        payload.bank,
        payload.vat_registered,
    )
    .execute(&state.pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [x] **Step 3: Створити placeholder файли для інших команд (щоб проект компілювався)**

Для кожного файлу (`dashboard.rs`, `counterparties.rs`, `documents.rs`, `payments.rs`, `tasks.rs`, `reports.rs`) — порожній модуль:

```rust
// src-tauri/src/commands/dashboard.rs
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_dashboard(_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({}))
}
```

Повторити для counterparties, documents, payments, tasks, reports з відповідними заглушками.

- [x] **Step 4: Перевірити компіляцію**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
```

Очікується: `Finished` без помилок.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/commands/
git commit -m "feat: add Tauri commands scaffold — settings + placeholder modules"
```

---

## Task 4: TypeScript типи та API клієнт

**Files:**
- Create: `src/lib/types.ts`
- Create: `src/lib/api.ts`

- [x] **Step 1: Створити `src/lib/types.ts`**

```typescript
// Навігація
export type NavScreen = "Dashboard" | "Documents" | "Counterparties" | "Payments" | "Reports" | "Tasks" | "Settings";
export type DocumentKind = "Invoice" | "Act" | "Waybill";
export type DocumentStatus = "Draft" | "Issued" | "Signed" | "Paid" | "Overdue" | "Partial";
export type Direction = "In" | "Out";
export type Priority = "High" | "Medium" | "Low";
export type TaskStatus = "Open" | "InProgress" | "Done" | "Cancelled";

// Shell
export interface CompanyInfo {
  id: string;
  full_name: string;
  short_name: string;
  edrpou: string;
  ipn: string;
  address: string;
  director: string;
  iban: string;
  bank: string;
  vat_registered: boolean;
}

// Dashboard
export interface DashboardMetrics {
  revenue_month: string;
  expenses_month: string;
  net_month: string;
  outstanding: string;
  overdue: string;
}

export interface ChartBar {
  rev_h: number;
  exp_h: number;
  month: string;
}

export interface JournalRow {
  date: string;
  id: string;
  operation: string;
  counterparty: string;
  debit_str: string;
  credit_str: string;
  is_credit: boolean;
  status_label: string;
}

export interface DashboardData {
  metrics: DashboardMetrics;
  chart_bars: ChartBar[];
  journal: JournalRow[];
}

// Documents
export interface DocumentItem {
  id: string;
  kind: DocumentKind;
  number: string;
  date: string;
  counterparty: string;
  amount_str: string;
  status: DocumentStatus;
  linked_id: string;
}

export interface DocumentDraftItem {
  description: string;
  unit: string;
  quantity: string;
  price: string;
}

export interface DocumentDraft {
  id: string;
  kind: DocumentKind;
  counterparty_id: string;
  counterparty_name: string;
  number: string;
  date: string;
  notes: string;
  items: DocumentDraftItem[];
}

export interface DocumentsData {
  items: DocumentItem[];
  total: number;
}

// Counterparties
export interface CounterpartyItem {
  id: string;
  name: string;
  edrpou: string;
  kind: string;
  balance_str: string;
  doc_count: number;
  overdue_count: number;
}

export interface CounterpartyDetail {
  id: string;
  name: string;
  edrpou: string;
  ipn: string;
  iban: string;
  address: string;
  phone: string;
  email: string;
  balance_str: string;
  doc_count: number;
  documents: DocumentItem[];
  payments: PaymentItem[];
}

// Payments
export interface PaymentItem {
  id: string;
  date: string;
  counterparty: string;
  amount_str: string;
  direction: Direction;
  matched_doc: string;
  account: string;
}

export interface PaymentsData {
  items: PaymentItem[];
  incoming_str: string;
  outgoing_str: string;
  net_str: string;
  unmatched_count: number;
}

// Tasks
export interface TaskItem {
  id: string;
  title: string;
  description: string;
  due_date: string;
  done: boolean;
  status: TaskStatus;
  priority: Priority;
}

export interface TasksData {
  open: TaskItem[];
  done: TaskItem[];
}

// Reports
export interface ReportMetrics {
  revenue: string;
  expenses: string;
  profit: string;
  margin: string;
}

export interface ExpenseCategory {
  label: string;
  amount_str: string;
  percent: number;
}

export interface ReportsData {
  metrics: ReportMetrics;
  chart_bars: ChartBar[];
  categories: ExpenseCategory[];
}

// Settings
export interface SettingsData {
  company: CompanyInfo;
}
```

- [x] **Step 2: Створити `src/lib/api.ts`**

```typescript
import { invoke } from "@tauri-apps/api/core";
import type {
  SettingsData, CompanyInfo,
  DashboardData,
  DocumentsData, DocumentDraft,
  CounterpartyItem, CounterpartyDetail,
  PaymentsData,
  TasksData, TaskItem,
  ReportsData,
} from "./types";

// Settings
export const getSettings = () => invoke<SettingsData>("get_settings");
export const saveCompany = (payload: CompanyInfo) => invoke<void>("save_company", { payload });

// Dashboard
export const getDashboard = () => invoke<DashboardData>("get_dashboard");

// Documents
export const listDocuments = (params: { kind?: string; status?: string; search?: string; page?: number }) =>
  invoke<DocumentsData>("list_documents", { params });
export const getDocument = (id: string) => invoke<DocumentDraft>("get_document", { id });
export const saveDocument = (draft: DocumentDraft) => invoke<string>("save_document", { draft });
export const changeDocumentStatus = (id: string, status: string) =>
  invoke<void>("change_document_status", { id, status });
export const deleteDocument = (id: string) => invoke<void>("delete_document", { id });

// Counterparties
export const listCounterparties = (search?: string) =>
  invoke<CounterpartyItem[]>("list_counterparties", { search: search ?? "" });
export const getCounterpartyDetail = (id: string) =>
  invoke<CounterpartyDetail>("get_counterparty_detail", { id });
export const saveCounterparty = (data: Partial<CounterpartyDetail>) =>
  invoke<string>("save_counterparty", { data });
export const deleteCounterparty = (id: string) => invoke<void>("delete_counterparty", { id });

// Payments
export const listPayments = () => invoke<PaymentsData>("list_payments");
export const importPaymentsCsv = (path: string) => invoke<number>("import_payments_csv", { path });

// Tasks
export const listTasks = () => invoke<TasksData>("list_tasks");
export const saveTask = (task: Partial<TaskItem>) => invoke<string>("save_task", { task });
export const setTaskStatus = (id: string, status: string) =>
  invoke<void>("set_task_status", { id, status });

// Reports
export const getReports = (year: number, month: number) =>
  invoke<ReportsData>("get_reports", { year, month });
```

- [x] **Step 3: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat: add TypeScript types and Tauri API client"
```

---

## Task 5: Design System CSS та Shell компонент

**Files:**
- Create: `src/lib/styles/tokens.css`
- Create: `src/lib/styles/global.css`
- Create: `src/lib/components/Shell.svelte`
- Create: `src/lib/components/Sidebar.svelte`
- Create: `src/app.svelte`

- [x] **Step 1: Створити `src/lib/styles/tokens.css`**

```css
:root {
  /* Colors (з design-tokens.slint) */
  --color-bg: #f8f7f4;
  --color-surface: #ffffff;
  --color-border: #e5e2dc;
  --color-sidebar-bg: #f0ede8;
  --color-primary: #3d75f4;
  --color-primary-light: #e8f0fe;
  --color-text-main: #1a1916;
  --color-text-sub: #4a4742;
  --color-text-muted: #8a8784;
  --color-success: #2d9c5a;
  --color-warning: #c97b1a;
  --color-danger: #d93025;
  --color-row-alt: #f4f2ef;

  /* Typography */
  --font-family: -apple-system, BlinkMacSystemFont, "Inter", "Segoe UI", sans-serif;
  --font-size-xs: 11px;
  --font-size-sm: 12px;
  --font-size-base: 13px;
  --font-size-md: 14px;
  --font-size-lg: 16px;
  --font-size-xl: 20px;

  /* Spacing */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;

  /* Radius */
  --radius-sm: 4px;
  --radius-md: 6px;
  --radius-lg: 8px;

  /* Sidebar */
  --sidebar-width: 220px;
}
```

- [x] **Step 2: Створити `src/lib/styles/global.css`**

```css
@import "./tokens.css";

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: var(--font-family);
  font-size: var(--font-size-base);
  color: var(--color-text-main);
  background: var(--color-bg);
  -webkit-font-smoothing: antialiased;
  overflow: hidden;
  height: 100vh;
}

button {
  font-family: inherit;
  cursor: pointer;
  border: none;
  background: none;
}

input, textarea, select {
  font-family: inherit;
  font-size: var(--font-size-base);
  color: var(--color-text-main);
}

.btn-primary {
  background: var(--color-primary);
  color: #fff;
  padding: 6px 14px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  font-weight: 500;
  transition: opacity 0.1s;
}
.btn-primary:hover { opacity: 0.88; }

.btn-ghost {
  color: var(--color-text-sub);
  padding: 6px 12px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
}
.btn-ghost:hover { background: var(--color-row-alt); }

.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 7px;
  border-radius: 10px;
  font-size: var(--font-size-xs);
  font-weight: 500;
}
.badge-blue { background: var(--color-primary-light); color: var(--color-primary); }
.badge-green { background: #d4edda; color: var(--color-success); }
.badge-orange { background: #fdebd0; color: var(--color-warning); }
.badge-red { background: #fde8e6; color: var(--color-danger); }
.badge-gray { background: var(--color-row-alt); color: var(--color-text-muted); }
```

- [x] **Step 3: Створити `src/lib/components/Sidebar.svelte`**

```svelte
<script lang="ts">
  import type { NavScreen } from "../types";

  let { active, onNavigate }: { active: NavScreen; onNavigate: (s: NavScreen) => void } = $props();

  const items: { screen: NavScreen; label: string }[] = [
    { screen: "Dashboard", label: "Головна" },
    { screen: "Documents", label: "Документи" },
    { screen: "Counterparties", label: "Контрагенти" },
    { screen: "Payments", label: "Платежі" },
    { screen: "Reports", label: "Звіти" },
    { screen: "Tasks", label: "Задачі" },
    { screen: "Settings", label: "Налаштування" },
  ];
</script>

<nav class="sidebar">
  <div class="logo">Acta</div>
  {#each items as item}
    <button
      class="nav-item"
      class:active={active === item.screen}
      onclick={() => onNavigate(item.screen)}
    >
      {item.label}
    </button>
  {/each}
</nav>

<style>
  .sidebar {
    width: var(--sidebar-width);
    background: var(--color-sidebar-bg);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    padding: var(--space-4);
    gap: var(--space-1);
    height: 100vh;
    flex-shrink: 0;
  }
  .logo {
    font-size: var(--font-size-lg);
    font-weight: 700;
    color: var(--color-primary);
    padding: var(--space-3) var(--space-2);
    margin-bottom: var(--space-4);
  }
  .nav-item {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    text-align: left;
    color: var(--color-text-sub);
    font-size: var(--font-size-sm);
    width: 100%;
    transition: background 0.1s, color 0.1s;
  }
  .nav-item:hover { background: var(--color-border); }
  .nav-item.active {
    background: var(--color-primary);
    color: #fff;
  }
</style>
```

- [x] **Step 4: Створити `src/app.svelte`**

```svelte
<script lang="ts">
  import "../lib/styles/global.css";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Dashboard from "./screens/Dashboard.svelte";
  import Documents from "./screens/Documents.svelte";
  import Counterparties from "./screens/Counterparties.svelte";
  import Payments from "./screens/Payments.svelte";
  import Tasks from "./screens/Tasks.svelte";
  import Reports from "./screens/Reports.svelte";
  import Settings from "./screens/Settings.svelte";
  import type { NavScreen } from "./lib/types";

  let activeScreen: NavScreen = $state("Dashboard");
</script>

<div class="app">
  <Sidebar active={activeScreen} onNavigate={(s) => (activeScreen = s)} />
  <main class="content">
    {#if activeScreen === "Dashboard"}
      <Dashboard />
    {:else if activeScreen === "Documents"}
      <Documents />
    {:else if activeScreen === "Counterparties"}
      <Counterparties />
    {:else if activeScreen === "Payments"}
      <Payments />
    {:else if activeScreen === "Tasks"}
      <Tasks />
    {:else if activeScreen === "Reports"}
      <Reports />
    {:else if activeScreen === "Settings"}
      <Settings />
    {/if}
  </main>
</div>

<style>
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  .content {
    flex: 1;
    overflow-y: auto;
    background: var(--color-bg);
  }
</style>
```

- [x] **Step 5: Створити заглушки для всіх screens (щоб компілювалось)**

Для кожного `src/screens/*.svelte`:
```svelte
<div class="screen-placeholder">
  <h1>Назва екрану</h1>
</div>
```

- [x] **Step 6: Запустити та перевірити що вікно відкривається з sidebar**

```bash
npm run dev
```

Очікується: вікно відкрилось, sidebar видно, навігація між секціями працює.

- [x] **Step 7: Commit**

```bash
git add src/
git commit -m "feat: add Svelte app shell with sidebar navigation and design tokens"
```

---

## Task 6: Settings Screen

**Files:**
- Modify: `src-tauri/src/commands/settings.rs` (вже створено в Task 3)
- Create: `src/screens/Settings.svelte`

- [x] **Step 1: Створити `src/screens/Settings.svelte`**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { getSettings, saveCompany } from "../lib/api";
  import type { CompanyInfo } from "../lib/types";

  let company: CompanyInfo | null = $state(null);
  let saving = $state(false);
  let saved = $state(false);

  onMount(async () => {
    const data = await getSettings();
    company = data.company;
  });

  async function handleSave() {
    if (!company) return;
    saving = true;
    try {
      await saveCompany(company);
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } finally {
      saving = false;
    }
  }
</script>

<div class="settings">
  <header class="page-header">
    <h1>Налаштування</h1>
  </header>

  {#if company}
    <section class="section">
      <h2>Компанія</h2>
      <div class="form-grid">
        <label>
          Повна назва
          <input bind:value={company.full_name} />
        </label>
        <label>
          Коротка назва
          <input bind:value={company.short_name} />
        </label>
        <label>
          ЄДРПОУ
          <input bind:value={company.edrpou} />
        </label>
        <label>
          ІПН
          <input bind:value={company.ipn} />
        </label>
        <label>
          Адреса
          <input bind:value={company.address} />
        </label>
        <label>
          Директор
          <input bind:value={company.director} />
        </label>
        <label>
          IBAN
          <input bind:value={company.iban} />
        </label>
        <label>
          Банк
          <input bind:value={company.bank} />
        </label>
      </div>
      <label class="checkbox-row">
        <input type="checkbox" bind:checked={company.vat_registered} />
        Платник ПДВ
      </label>
      <div class="actions">
        <button class="btn-primary" onclick={handleSave} disabled={saving}>
          {saving ? "Збереження..." : saved ? "Збережено ✓" : "Зберегти"}
        </button>
      </div>
    </section>
  {:else}
    <p>Завантаження...</p>
  {/if}
</div>

<style>
  .settings { padding: var(--space-6); max-width: 800px; }
  .page-header { margin-bottom: var(--space-6); }
  h1 { font-size: var(--font-size-xl); font-weight: 600; }
  h2 { font-size: var(--font-size-md); font-weight: 600; margin-bottom: var(--space-4); }
  .section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
    margin-bottom: var(--space-4);
  }
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--font-size-sm);
    color: var(--color-text-sub);
  }
  input {
    padding: 7px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text-main);
  }
  input:focus { outline: 2px solid var(--color-primary); border-color: transparent; }
  .checkbox-row {
    flex-direction: row;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }
  .actions { display: flex; gap: var(--space-2); }
</style>
```

- [x] **Step 2: Запустити та перевірити Settings**

```bash
npm run dev
```

Перейти на Settings — форма завантажується, редагується, кнопка "Зберегти" спрацьовує.

- [x] **Step 3: Commit**

```bash
git add src/screens/Settings.svelte
git commit -m "feat: implement Settings screen with company form"
```

---

## Task 7: Dashboard Command + Screen

**Files:**
- Modify: `src-tauri/src/commands/dashboard.rs`
- Create: `src/lib/components/KpiCard.svelte`
- Create: `src/lib/components/BarChart.svelte`
- Modify: `src/screens/Dashboard.svelte`

- [x] **Step 1: Реалізувати `src-tauri/src/commands/dashboard.rs`**

```rust
use crate::state::AppState;
use crate::db;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct DashboardResponse {
    pub metrics: Metrics,
    pub chart_bars: Vec<ChartBar>,
    pub journal: Vec<JournalRow>,
}

#[derive(Serialize)]
pub struct Metrics {
    pub revenue_month: String,
    pub expenses_month: String,
    pub net_month: String,
    pub outstanding: String,
    pub overdue: String,
}

#[derive(Serialize)]
pub struct ChartBar {
    pub rev_h: f32,
    pub exp_h: f32,
    pub month: String,
}

#[derive(Serialize)]
pub struct JournalRow {
    pub date: String,
    pub id: String,
    pub operation: String,
    pub counterparty: String,
    pub debit_str: String,
    pub credit_str: String,
    pub is_credit: bool,
}

#[tauri::command]
pub async fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardResponse, String> {
    let company_id = state.company_id().await;

    let (kpi, months_rev, months_exp, recent) = tokio::join!(
        db::dashboard::get_kpi_summary(&state.pool, company_id),
        db::dashboard::revenue_by_month(&state.pool, company_id),
        db::dashboard::expenses_by_month(&state.pool, company_id),
        db::dashboard::get_recent_acts(&state.pool, company_id),
    );

    let kpi = kpi.map_err(|e| e.to_string())?;
    let months_rev = months_rev.map_err(|e| e.to_string())?;
    let months_exp = months_exp.map_err(|e| e.to_string())?;
    let recent = recent.map_err(|e| e.to_string())?;

    let max_rev = months_rev.iter().map(|m| m.revenue).fold(rust_decimal::Decimal::ZERO, |a, b| a.max(b));
    let max_exp = months_exp.iter().map(|m| m.expenses).fold(rust_decimal::Decimal::ZERO, |a, b| a.max(b));
    let max_val = max_rev.max(max_exp);

    let chart_bars = months_rev.iter().zip(months_exp.iter()).map(|(r, e)| {
        let rev_h = if max_val.is_zero() { 0.0 } else { (r.revenue / max_val).to_f32_saturating() };
        let exp_h = if max_val.is_zero() { 0.0 } else { (e.expenses / max_val).to_f32_saturating() };
        ChartBar {
            rev_h,
            exp_h,
            month: r.month.format("%b").to_string(),
        }
    }).collect();

    let journal = recent.into_iter().map(|a| JournalRow {
        date: a.date.format("%d.%m.%Y").to_string(),
        id: a.id.to_string(),
        operation: format!("Акт {}", a.number),
        counterparty: a.counterparty_name,
        debit_str: format!("{:.2}", a.amount),
        credit_str: String::new(),
        is_credit: false,
    }).collect();

    Ok(DashboardResponse {
        metrics: Metrics {
            revenue_month: format!("{:.2}", kpi.revenue_month),
            expenses_month: format!("{:.2}", kpi.expenses_month),
            net_month: format!("{:.2}", kpi.net_month),
            outstanding: format!("{:.2}", kpi.outstanding),
            overdue: format!("{:.2}", kpi.overdue),
        },
        chart_bars,
        journal,
    })
}
```

- [x] **Step 2: Створити `src/lib/components/KpiCard.svelte`**

```svelte
<script lang="ts">
  let { label, value, sub = "" }: { label: string; value: string; sub?: string } = $props();
</script>

<div class="kpi-card">
  <span class="label">{label}</span>
  <span class="value">{value}</span>
  {#if sub}<span class="sub">{sub}</span>{/if}
</div>

<style>
  .kpi-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .label { font-size: var(--font-size-xs); color: var(--color-text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
  .value { font-size: var(--font-size-xl); font-weight: 600; }
  .sub { font-size: var(--font-size-xs); color: var(--color-text-muted); }
</style>
```

- [x] **Step 3: Реалізувати `src/screens/Dashboard.svelte`**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { getDashboard } from "../lib/api";
  import KpiCard from "../lib/components/KpiCard.svelte";
  import type { DashboardData } from "../lib/types";

  let data: DashboardData | null = $state(null);

  onMount(async () => {
    data = await getDashboard();
  });
</script>

<div class="dashboard">
  <header class="page-header">
    <h1>Головна</h1>
  </header>

  {#if data}
    <div class="kpi-strip">
      <KpiCard label="Дохід (місяць)" value={data.metrics.revenue_month} />
      <KpiCard label="Витрати (місяць)" value={data.metrics.expenses_month} />
      <KpiCard label="Прибуток" value={data.metrics.net_month} />
      <KpiCard label="До оплати" value={data.metrics.outstanding} />
      <KpiCard label="Прострочено" value={data.metrics.overdue} />
    </div>

    <div class="main-grid">
      <section class="card">
        <h2>Останні операції</h2>
        <table class="table">
          <thead>
            <tr><th>Дата</th><th>Операція</th><th>Контрагент</th><th>Дебет</th><th>Кредит</th></tr>
          </thead>
          <tbody>
            {#each data.journal as row}
              <tr>
                <td>{row.date}</td>
                <td>{row.operation}</td>
                <td>{row.counterparty}</td>
                <td>{row.is_credit ? "" : row.debit_str}</td>
                <td>{row.is_credit ? row.credit_str : ""}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    </div>
  {:else}
    <p class="loading">Завантаження...</p>
  {/if}
</div>

<style>
  .dashboard { padding: var(--space-6); }
  .page-header { margin-bottom: var(--space-5); }
  h1 { font-size: var(--font-size-xl); font-weight: 600; }
  .kpi-strip {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: var(--space-3);
    margin-bottom: var(--space-5);
  }
  .main-grid { display: grid; gap: var(--space-4); }
  .card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
  }
  h2 { font-size: var(--font-size-md); font-weight: 600; margin-bottom: var(--space-4); }
  .table { width: 100%; border-collapse: collapse; }
  .table th { font-size: var(--font-size-xs); color: var(--color-text-muted); text-align: left; padding: var(--space-2); border-bottom: 1px solid var(--color-border); }
  .table td { padding: var(--space-2); font-size: var(--font-size-sm); border-bottom: 1px solid var(--color-row-alt); }
  .loading { color: var(--color-text-muted); padding: var(--space-6); }
</style>
```

- [x] **Step 4: Перевірити Dashboard**

```bash
npm run dev
```

Dashboard відкривається, показує KPI та таблицю.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/commands/dashboard.rs src/screens/Dashboard.svelte src/lib/components/KpiCard.svelte
git commit -m "feat: implement Dashboard command and screen with KPI and journal"
```

---

## Task 8: Counterparties Command + Screen

**Files:**
- Modify: `src-tauri/src/commands/counterparties.rs`
- Create: `src/screens/Counterparties.svelte`

- [x] **Step 1: Реалізувати `src-tauri/src/commands/counterparties.rs`**

```rust
use crate::{db, state::AppState};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Serialize)]
pub struct CounterpartyItem {
    pub id: String,
    pub name: String,
    pub edrpou: String,
    pub kind: String,
    pub balance_str: String,
    pub doc_count: i64,
    pub overdue_count: i64,
}

#[derive(Serialize)]
pub struct CounterpartyDetailResponse {
    pub id: String,
    pub name: String,
    pub edrpou: String,
    pub ipn: String,
    pub iban: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub balance_str: String,
    pub doc_count: i64,
}

#[derive(Deserialize)]
pub struct SaveCounterpartyPayload {
    pub id: Option<String>,
    pub name: String,
    pub edrpou: String,
    pub ipn: String,
    pub iban: String,
    pub address: String,
    pub phone: String,
    pub email: String,
}

#[tauri::command]
pub async fn list_counterparties(
    state: State<'_, AppState>,
    search: String,
) -> Result<Vec<CounterpartyItem>, String> {
    let company_id = state.company_id().await;
    let rows = db::counterparties::search(&state.pool, company_id, &search)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|c| CounterpartyItem {
        id: c.id.to_string(),
        name: c.name.clone(),
        edrpou: c.edrpou.clone().unwrap_or_default(),
        kind: "ЮО".to_string(),
        balance_str: "0.00".to_string(),
        doc_count: 0,
        overdue_count: 0,
    }).collect())
}

#[tauri::command]
pub async fn get_counterparty_detail(
    state: State<'_, AppState>,
    id: String,
) -> Result<CounterpartyDetailResponse, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let c = db::counterparties::get_by_id(&state.pool, uuid)
        .await
        .map_err(|e| e.to_string())?;

    Ok(CounterpartyDetailResponse {
        id: c.id.to_string(),
        name: c.name,
        edrpou: c.edrpou.unwrap_or_default(),
        ipn: c.ipn.unwrap_or_default(),
        iban: c.iban.unwrap_or_default(),
        address: c.address.unwrap_or_default(),
        phone: c.phone.unwrap_or_default(),
        email: c.email.unwrap_or_default(),
        balance_str: "0.00".to_string(),
        doc_count: 0,
    })
}

#[tauri::command]
pub async fn save_counterparty(
    state: State<'_, AppState>,
    data: SaveCounterpartyPayload,
) -> Result<String, String> {
    let company_id = state.company_id().await;

    if let Some(id_str) = &data.id {
        let id = Uuid::parse_str(id_str).map_err(|e| e.to_string())?;
        let payload = crate::models::counterparty::UpdateCounterparty {
            name: data.name,
            edrpou: Some(data.edrpou),
            ipn: Some(data.ipn),
            iban: Some(data.iban),
            address: Some(data.address),
            phone: Some(data.phone),
            email: Some(data.email),
            ..Default::default()
        };
        db::counterparties::update(&state.pool, id, payload)
            .await
            .map_err(|e| e.to_string())?;
        Ok(id_str.clone())
    } else {
        let payload = crate::models::counterparty::NewCounterparty {
            company_id,
            name: data.name,
            edrpou: Some(data.edrpou),
            ipn: Some(data.ipn),
            iban: Some(data.iban),
            address: Some(data.address),
            phone: Some(data.phone),
            email: Some(data.email),
            ..Default::default()
        };
        let id = db::counterparties::create(&state.pool, payload)
            .await
            .map_err(|e| e.to_string())?;
        Ok(id.to_string())
    }
}

#[tauri::command]
pub async fn delete_counterparty(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    db::counterparties::archive(&state.pool, uuid)
        .await
        .map_err(|e| e.to_string())
}
```

- [x] **Step 2: Реалізувати `src/screens/Counterparties.svelte`**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { listCounterparties, getCounterpartyDetail, saveCounterparty } from "../lib/api";
  import type { CounterpartyItem, CounterpartyDetail } from "../lib/types";

  let items: CounterpartyItem[] = $state([]);
  let selected: CounterpartyDetail | null = $state(null);
  let search = $state("");
  let showEditor = $state(false);
  let editForm: Partial<CounterpartyDetail> = $state({});

  onMount(() => load());

  async function load() {
    items = await listCounterparties(search);
  }

  async function selectCounterparty(id: string) {
    selected = await getCounterpartyDetail(id);
  }

  function openEditor(detail?: CounterpartyDetail) {
    editForm = detail ? { ...detail } : {};
    showEditor = true;
  }

  async function handleSave() {
    await saveCounterparty(editForm);
    showEditor = false;
    await load();
  }

  let searchTimer: ReturnType<typeof setTimeout>;
  function onSearch(e: Event) {
    search = (e.target as HTMLInputElement).value;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(load, 300);
  }
</script>

<div class="counterparties">
  <header class="page-header">
    <h1>Контрагенти</h1>
    <button class="btn-primary" onclick={() => openEditor()}>+ Новий</button>
  </header>

  <div class="layout">
    <!-- List -->
    <aside class="list-panel">
      <input class="search" placeholder="Пошук..." oninput={onSearch} value={search} />
      <div class="list">
        {#each items as item}
          <button
            class="list-item"
            class:active={selected?.id === item.id}
            onclick={() => selectCounterparty(item.id)}
          >
            <div class="item-name">{item.name}</div>
            <div class="item-meta">{item.edrpou} · {item.doc_count} документів</div>
          </button>
        {/each}
      </div>
    </aside>

    <!-- Detail -->
    <section class="detail-panel">
      {#if selected}
        <div class="detail-header">
          <h2>{selected.name}</h2>
          <button class="btn-ghost" onclick={() => openEditor(selected)}>Редагувати</button>
        </div>
        <div class="detail-grid">
          <div><span class="field-label">ЄДРПОУ</span><span>{selected.edrpou}</span></div>
          <div><span class="field-label">ІПН</span><span>{selected.ipn}</span></div>
          <div><span class="field-label">IBAN</span><span>{selected.iban}</span></div>
          <div><span class="field-label">Адреса</span><span>{selected.address}</span></div>
          <div><span class="field-label">Телефон</span><span>{selected.phone}</span></div>
          <div><span class="field-label">Email</span><span>{selected.email}</span></div>
        </div>
      {:else}
        <div class="empty">Оберіть контрагента</div>
      {/if}
    </section>
  </div>
</div>

<!-- Editor Modal -->
{#if showEditor}
  <div class="modal-backdrop" onclick={() => (showEditor = false)}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{editForm.id ? "Редагувати" : "Новий"} контрагент</h3>
      <div class="form-grid">
        <label>Назва <input bind:value={editForm.name} /></label>
        <label>ЄДРПОУ <input bind:value={editForm.edrpou} /></label>
        <label>ІПН <input bind:value={editForm.ipn} /></label>
        <label>IBAN <input bind:value={editForm.iban} /></label>
        <label>Адреса <input bind:value={editForm.address} /></label>
        <label>Телефон <input bind:value={editForm.phone} /></label>
        <label>Email <input bind:value={editForm.email} /></label>
      </div>
      <div class="modal-actions">
        <button class="btn-ghost" onclick={() => (showEditor = false)}>Скасувати</button>
        <button class="btn-primary" onclick={handleSave}>Зберегти</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .counterparties { display: flex; flex-direction: column; height: 100%; }
  .page-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-5) var(--space-6); border-bottom: 1px solid var(--color-border); }
  h1 { font-size: var(--font-size-xl); font-weight: 600; }
  .layout { display: flex; flex: 1; overflow: hidden; }

  .list-panel { width: 340px; border-right: 1px solid var(--color-border); display: flex; flex-direction: column; }
  .search { padding: var(--space-3); border: none; border-bottom: 1px solid var(--color-border); width: 100%; background: var(--color-sidebar-bg); }
  .list { overflow-y: auto; flex: 1; }
  .list-item { display: block; width: 100%; text-align: left; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--color-row-alt); }
  .list-item:hover { background: var(--color-row-alt); }
  .list-item.active { background: var(--color-primary-light); }
  .item-name { font-size: var(--font-size-sm); font-weight: 500; }
  .item-meta { font-size: var(--font-size-xs); color: var(--color-text-muted); margin-top: 2px; }

  .detail-panel { flex: 1; padding: var(--space-6); overflow-y: auto; }
  .detail-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: var(--space-5); }
  h2 { font-size: var(--font-size-lg); font-weight: 600; }
  .detail-grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3); }
  .field-label { display: block; font-size: var(--font-size-xs); color: var(--color-text-muted); margin-bottom: 2px; }
  .empty { color: var(--color-text-muted); padding: var(--space-8); text-align: center; }

  .modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.3); display: flex; align-items: center; justify-content: center; z-index: 100; }
  .modal { background: var(--color-surface); border-radius: var(--radius-lg); padding: var(--space-6); width: 500px; }
  .modal h3 { font-size: var(--font-size-lg); font-weight: 600; margin-bottom: var(--space-5); }
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3); margin-bottom: var(--space-5); }
  .form-grid label { display: flex; flex-direction: column; gap: var(--space-1); font-size: var(--font-size-sm); color: var(--color-text-sub); }
  .form-grid input { padding: 7px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
  .modal-actions { display: flex; justify-content: flex-end; gap: var(--space-2); }
</style>
```

- [x] **Step 3: Перевірити Counterparties**

```bash
npm run dev
```

Список контрагентів, кліком відкривається деталь, кнопка "Редагувати" відкриває модальне вікно.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/commands/counterparties.rs src/screens/Counterparties.svelte
git commit -m "feat: implement Counterparties command and master-detail screen"
```

---

## Task 9: Documents Command + Screen

**Files:**
- Modify: `src-tauri/src/commands/documents.rs`
- Create: `src/screens/Documents.svelte`

- [x] **Step 1: Реалізувати `src-tauri/src/commands/documents.rs`**

```rust
use crate::{db, models, state::AppState};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Serialize)]
pub struct DocumentItem {
    pub id: String,
    pub kind: String,
    pub number: String,
    pub date: String,
    pub counterparty: String,
    pub amount_str: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct DocumentsResponse {
    pub items: Vec<DocumentItem>,
    pub total: usize,
}

#[derive(Deserialize)]
pub struct ListDocumentsParams {
    pub kind: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DraftItem {
    pub description: String,
    pub unit: String,
    pub quantity: String,
    pub price: String,
}

#[derive(Deserialize, Serialize)]
pub struct DocumentDraft {
    pub id: Option<String>,
    pub kind: String,
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub number: String,
    pub date: String,
    pub notes: String,
    pub items: Vec<DraftItem>,
}

#[tauri::command]
pub async fn list_documents(
    state: State<'_, AppState>,
    params: ListDocumentsParams,
) -> Result<DocumentsResponse, String> {
    let company_id = state.company_id().await;
    let search = params.search.as_deref().unwrap_or("");

    let kind = params.kind.as_deref().unwrap_or("Act");

    let items = match kind {
        "Invoice" => {
            let rows = db::invoices::list_filtered(&state.pool, company_id, search, None, 50, 0)
                .await.map_err(|e| e.to_string())?;
            rows.into_iter().map(|r| DocumentItem {
                id: r.id.to_string(),
                kind: "Invoice".to_string(),
                number: r.number,
                date: r.date.format("%d.%m.%Y").to_string(),
                counterparty: r.counterparty_name,
                amount_str: format!("{:.2}", r.amount),
                status: format!("{:?}", r.status),
            }).collect()
        }
        "Waybill" => {
            let rows = db::waybills::list_filtered(&state.pool, company_id, search, None, 50, 0)
                .await.map_err(|e| e.to_string())?;
            rows.into_iter().map(|r| DocumentItem {
                id: r.id.to_string(),
                kind: "Waybill".to_string(),
                number: r.number,
                date: r.date.format("%d.%m.%Y").to_string(),
                counterparty: r.counterparty_name,
                amount_str: format!("{:.2}", r.amount),
                status: format!("{:?}", r.status),
            }).collect()
        }
        _ => {
            let rows = db::acts::list_filtered(&state.pool, company_id, search, None, 50, 0)
                .await.map_err(|e| e.to_string())?;
            rows.into_iter().map(|r| DocumentItem {
                id: r.id.to_string(),
                kind: "Act".to_string(),
                number: r.number,
                date: r.date.format("%d.%m.%Y").to_string(),
                counterparty: r.counterparty_name,
                amount_str: format!("{:.2}", r.amount),
                status: format!("{:?}", r.status),
            }).collect()
        }
    };

    let total = items.len();
    Ok(DocumentsResponse { items, total })
}

#[tauri::command]
pub async fn change_document_status(
    state: State<'_, AppState>,
    id: String,
    kind: String,
    status: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    match kind.as_str() {
        "Invoice" => db::invoices::advance_status(&state.pool, uuid).await.map_err(|e| e.to_string()),
        "Waybill" => db::waybills::advance_status(&state.pool, uuid).await.map_err(|e| e.to_string()),
        _ => db::acts::advance_status(&state.pool, uuid).await.map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub async fn delete_document(
    state: State<'_, AppState>,
    id: String,
    kind: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    match kind.as_str() {
        "Invoice" => db::invoices::delete(&state.pool, uuid).await.map_err(|e| e.to_string()),
        "Waybill" => db::waybills::delete(&state.pool, uuid).await.map_err(|e| e.to_string()),
        _ => db::acts::delete(&state.pool, uuid).await.map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub async fn get_document(
    _state: State<'_, AppState>,
    _id: String,
) -> Result<DocumentDraft, String> {
    // TODO: implement full document load with items
    Err("Not implemented".to_string())
}

#[tauri::command]
pub async fn save_document(
    _state: State<'_, AppState>,
    _draft: DocumentDraft,
) -> Result<String, String> {
    // TODO: implement save with items
    Err("Not implemented".to_string())
}
```

- [x] **Step 2: Реалізувати `src/screens/Documents.svelte`**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { listDocuments, changeDocumentStatus, deleteDocument } from "../lib/api";
  import type { DocumentItem, DocumentKind } from "../lib/types";

  type Tab = "Act" | "Invoice" | "Waybill";
  const tabs: { key: Tab; label: string }[] = [
    { key: "Act", label: "Акти" },
    { key: "Invoice", label: "Рахунки" },
    { key: "Waybill", label: "Накладні" },
  ];

  let activeTab: Tab = $state("Act");
  let items: DocumentItem[] = $state([]);
  let search = $state("");
  let loading = $state(false);

  const statusLabels: Record<string, string> = {
    Draft: "Чернетка",
    Issued: "Виставлено",
    Signed: "Підписано",
    Paid: "Оплачено",
    Overdue: "Прострочено",
    Partial: "Часткова оплата",
  };

  const statusClasses: Record<string, string> = {
    Draft: "badge-gray",
    Issued: "badge-blue",
    Signed: "badge-green",
    Paid: "badge-green",
    Overdue: "badge-red",
    Partial: "badge-orange",
  };

  onMount(() => load());

  async function load() {
    loading = true;
    try {
      const data = await listDocuments({ kind: activeTab, search });
      items = data.items;
    } finally {
      loading = false;
    }
  }

  function switchTab(tab: Tab) {
    activeTab = tab;
    load();
  }

  let searchTimer: ReturnType<typeof setTimeout>;
  function onSearch(e: Event) {
    search = (e.target as HTMLInputElement).value;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(load, 300);
  }

  async function advanceStatus(item: DocumentItem) {
    await changeDocumentStatus(item.id, item.kind, item.status);
    await load();
  }

  async function handleDelete(item: DocumentItem) {
    if (!confirm(`Видалити ${item.number}?`)) return;
    await deleteDocument(item.id, item.kind);
    await load();
  }
</script>

<div class="documents">
  <header class="page-header">
    <h1>Документи</h1>
    <button class="btn-primary">+ Новий</button>
  </header>

  <div class="toolbar">
    <div class="tabs">
      {#each tabs as tab}
        <button
          class="tab"
          class:active={activeTab === tab.key}
          onclick={() => switchTab(tab.key)}
        >
          {tab.label}
        </button>
      {/each}
    </div>
    <input class="search-input" placeholder="Пошук..." oninput={onSearch} value={search} />
  </div>

  <div class="table-wrap">
    {#if loading}
      <p class="loading-msg">Завантаження...</p>
    {:else}
      <table class="table">
        <thead>
          <tr>
            <th>Номер</th>
            <th>Дата</th>
            <th>Контрагент</th>
            <th>Сума</th>
            <th>Статус</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each items as item}
            <tr>
              <td class="doc-number">{item.number}</td>
              <td>{item.date}</td>
              <td>{item.counterparty}</td>
              <td class="amount">{item.amount_str}</td>
              <td>
                <span class="badge {statusClasses[item.status] ?? 'badge-gray'}">
                  {statusLabels[item.status] ?? item.status}
                </span>
              </td>
              <td class="actions-cell">
                <button class="btn-ghost" onclick={() => advanceStatus(item)}>→</button>
                <button class="btn-ghost danger" onclick={() => handleDelete(item)}>✕</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if items.length === 0}
        <p class="empty-msg">Документів не знайдено</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .documents { display: flex; flex-direction: column; height: 100%; }
  .page-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-5) var(--space-6); border-bottom: 1px solid var(--color-border); }
  h1 { font-size: var(--font-size-xl); font-weight: 600; }

  .toolbar { display: flex; align-items: center; gap: var(--space-4); padding: var(--space-3) var(--space-6); border-bottom: 1px solid var(--color-border); background: var(--color-surface); }
  .tabs { display: flex; gap: 2px; }
  .tab { padding: var(--space-2) var(--space-4); border-radius: var(--radius-md); font-size: var(--font-size-sm); color: var(--color-text-sub); }
  .tab:hover { background: var(--color-row-alt); }
  .tab.active { background: var(--color-primary-light); color: var(--color-primary); font-weight: 500; }

  .search-input { margin-left: auto; padding: 6px 12px; border: 1px solid var(--color-border); border-radius: var(--radius-md); width: 220px; }

  .table-wrap { flex: 1; overflow-y: auto; padding: var(--space-4) var(--space-6); }
  .table { width: 100%; border-collapse: collapse; }
  .table th { font-size: var(--font-size-xs); color: var(--color-text-muted); text-align: left; padding: var(--space-2) var(--space-3); border-bottom: 1px solid var(--color-border); text-transform: uppercase; letter-spacing: 0.04em; }
  .table td { padding: var(--space-3); font-size: var(--font-size-sm); border-bottom: 1px solid var(--color-row-alt); }
  .table tr:hover td { background: var(--color-row-alt); }
  .doc-number { font-weight: 500; }
  .amount { font-variant-numeric: tabular-nums; }
  .actions-cell { display: flex; gap: var(--space-1); opacity: 0; }
  tr:hover .actions-cell { opacity: 1; }
  .danger { color: var(--color-danger); }
  .loading-msg, .empty-msg { text-align: center; padding: var(--space-8); color: var(--color-text-muted); }
</style>
```

- [x] **Step 3: Перевірити Documents**

```bash
npm run dev
```

Три вкладки, пошук, таблиця з документами, статус badges, дії.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/commands/documents.rs src/screens/Documents.svelte
git commit -m "feat: implement Documents command and tabbed screen"
```

---

## Task 10: Payments, Tasks, Reports

- [x] **Step 1: Реалізувати `src-tauri/src/commands/payments.rs`**

```rust
use crate::{db, state::AppState};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct PaymentItem {
    pub id: String,
    pub date: String,
    pub counterparty: String,
    pub amount_str: String,
    pub direction: String,
    pub account: String,
}

#[derive(Serialize)]
pub struct PaymentsResponse {
    pub items: Vec<PaymentItem>,
    pub incoming_str: String,
    pub outgoing_str: String,
    pub net_str: String,
    pub unmatched_count: i64,
}

#[tauri::command]
pub async fn list_payments(state: State<'_, AppState>) -> Result<PaymentsResponse, String> {
    let company_id = state.company_id().await;
    let (rows, kpi) = tokio::join!(
        db::payments::list(&state.pool, company_id),
        db::payments::payment_kpi(&state.pool, company_id),
    );
    let rows = rows.map_err(|e| e.to_string())?;
    let kpi = kpi.map_err(|e| e.to_string())?;

    let items = rows.into_iter().map(|p| PaymentItem {
        id: p.id.to_string(),
        date: p.date.format("%d.%m.%Y").to_string(),
        counterparty: p.counterparty_name.unwrap_or_default(),
        amount_str: format!("{:.2}", p.amount),
        direction: format!("{:?}", p.direction),
        account: String::new(),
    }).collect();

    Ok(PaymentsResponse {
        items,
        incoming_str: format!("{:.2}", kpi.total_in),
        outgoing_str: format!("{:.2}", kpi.total_out),
        net_str: format!("{:.2}", kpi.total_in - kpi.total_out),
        unmatched_count: kpi.unmatched_count,
    })
}

#[tauri::command]
pub async fn import_payments_csv(
    _state: State<'_, AppState>,
    _path: String,
) -> Result<usize, String> {
    // TODO: wire до import/bank parsers
    Ok(0)
}
```

- [x] **Step 2: Реалізувати `src-tauri/src/commands/tasks.rs`**

```rust
use crate::{db, models, state::AppState};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Serialize)]
pub struct TaskItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub due_date: String,
    pub done: bool,
    pub status: String,
    pub priority: String,
}

#[derive(Serialize)]
pub struct TasksResponse {
    pub open: Vec<TaskItem>,
    pub done: Vec<TaskItem>,
}

fn task_to_item(t: &crate::models::task::Task) -> TaskItem {
    TaskItem {
        id: t.id.to_string(),
        title: t.title.clone(),
        description: t.description.clone().unwrap_or_default(),
        due_date: t.due_date.map(|d| d.format("%d.%m.%Y").to_string()).unwrap_or_default(),
        done: matches!(t.status, crate::models::task::TaskStatus::Done),
        status: format!("{:?}", t.status),
        priority: format!("{:?}", t.priority),
    }
}

#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<TasksResponse, String> {
    let company_id = state.company_id().await;
    let all = db::tasks::list_all(&state.pool, company_id)
        .await.map_err(|e| e.to_string())?;

    let open = all.iter().filter(|t| !matches!(t.status, crate::models::task::TaskStatus::Done | crate::models::task::TaskStatus::Cancelled))
        .map(task_to_item).collect();
    let done = all.iter().filter(|t| matches!(t.status, crate::models::task::TaskStatus::Done))
        .map(task_to_item).collect();

    Ok(TasksResponse { open, done })
}

#[derive(Deserialize)]
pub struct SaveTaskPayload {
    pub id: Option<String>,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub due_date: Option<String>,
}

#[tauri::command]
pub async fn save_task(
    state: State<'_, AppState>,
    task: SaveTaskPayload,
) -> Result<String, String> {
    let company_id = state.company_id().await;
    let priority = match task.priority.as_str() {
        "High" => models::task::TaskPriority::High,
        "Low" => models::task::TaskPriority::Low,
        _ => models::task::TaskPriority::Medium,
    };

    if let Some(id_str) = &task.id {
        let id = Uuid::parse_str(id_str).map_err(|e| e.to_string())?;
        db::tasks::update(&state.pool, id, models::task::NewTask {
            company_id,
            title: task.title,
            description: Some(task.description),
            priority,
            due_date: None,
            reminder_at: None,
        }).await.map_err(|e| e.to_string())?;
        Ok(id_str.clone())
    } else {
        let id = db::tasks::create(&state.pool, models::task::NewTask {
            company_id,
            title: task.title,
            description: Some(task.description),
            priority,
            due_date: None,
            reminder_at: None,
        }).await.map_err(|e| e.to_string())?;
        Ok(id.to_string())
    }
}

#[tauri::command]
pub async fn set_task_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let s = match status.as_str() {
        "Done" => models::task::TaskStatus::Done,
        "InProgress" => models::task::TaskStatus::InProgress,
        "Cancelled" => models::task::TaskStatus::Cancelled,
        _ => models::task::TaskStatus::Open,
    };
    db::tasks::set_status(&state.pool, uuid, s)
        .await.map_err(|e| e.to_string())
}
```

- [x] **Step 3: Реалізувати `src-tauri/src/commands/reports.rs`**

```rust
use crate::{db, state::AppState};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ReportsResponse {
    pub metrics: Metrics,
    pub chart_bars: Vec<ChartBar>,
    pub categories: Vec<ExpenseCategory>,
}

#[derive(Serialize)]
pub struct Metrics {
    pub revenue: String,
    pub expenses: String,
    pub profit: String,
}

#[derive(Serialize)]
pub struct ChartBar { pub rev_h: f32, pub exp_h: f32, pub month: String }

#[derive(Serialize)]
pub struct ExpenseCategory { pub label: String, pub amount_str: String, pub percent: f64 }

#[tauri::command]
pub async fn get_reports(
    state: State<'_, AppState>,
    _year: i32,
    _month: u32,
) -> Result<ReportsResponse, String> {
    let company_id = state.company_id().await;
    let (kpi, cats) = tokio::join!(
        db::dashboard::get_kpi_summary(&state.pool, company_id),
        db::dashboard::category_breakdown(&state.pool, company_id),
    );
    let kpi = kpi.map_err(|e| e.to_string())?;
    let cats = cats.map_err(|e| e.to_string())?;

    let total_exp = kpi.expenses_month;
    let categories = cats.into_iter().map(|c| {
        let pct = if total_exp.is_zero() { 0.0 } else { (c.amount / total_exp).to_f64_saturating() * 100.0 };
        ExpenseCategory {
            label: c.category_name,
            amount_str: format!("{:.2}", c.amount),
            percent: pct,
        }
    }).collect();

    Ok(ReportsResponse {
        metrics: Metrics {
            revenue: format!("{:.2}", kpi.revenue_month),
            expenses: format!("{:.2}", kpi.expenses_month),
            profit: format!("{:.2}", kpi.net_month),
        },
        chart_bars: vec![],
        categories,
    })
}
```

- [x] **Step 4: Реалізувати три Svelte screens — Payments, Tasks, Reports**

`src/screens/Payments.svelte`:
```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { listPayments } from "../lib/api";
  import type { PaymentsData } from "../lib/types";

  let data: PaymentsData | null = $state(null);
  onMount(async () => { data = await listPayments(); });
</script>

<div class="screen">
  <header class="page-header"><h1>Платежі</h1></header>
  {#if data}
    <div class="kpi-strip">
      <div class="kpi"><span class="kpi-label">Надходження</span><span class="kpi-val">{data.incoming_str}</span></div>
      <div class="kpi"><span class="kpi-label">Списання</span><span class="kpi-val">{data.outgoing_str}</span></div>
      <div class="kpi"><span class="kpi-label">Нетто</span><span class="kpi-val">{data.net_str}</span></div>
    </div>
    <div class="table-wrap">
      <table class="table">
        <thead><tr><th>Дата</th><th>Контрагент</th><th>Сума</th><th>Тип</th></tr></thead>
        <tbody>
          {#each data.items as p}
            <tr>
              <td>{p.date}</td>
              <td>{p.counterparty}</td>
              <td class:income={p.direction === "In"} class:expense={p.direction === "Out"}>{p.amount_str}</td>
              <td><span class="badge {p.direction === 'In' ? 'badge-green' : 'badge-orange'}">{p.direction === "In" ? "Надходження" : "Списання"}</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .screen { display: flex; flex-direction: column; height: 100%; }
  .page-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-5) var(--space-6); border-bottom: 1px solid var(--color-border); }
  h1 { font-size: var(--font-size-xl); font-weight: 600; }
  .kpi-strip { display: flex; gap: var(--space-4); padding: var(--space-4) var(--space-6); border-bottom: 1px solid var(--color-border); }
  .kpi { display: flex; flex-direction: column; gap: 2px; }
  .kpi-label { font-size: var(--font-size-xs); color: var(--color-text-muted); }
  .kpi-val { font-size: var(--font-size-lg); font-weight: 600; }
  .table-wrap { flex: 1; overflow-y: auto; padding: var(--space-4) var(--space-6); }
  .table { width: 100%; border-collapse: collapse; }
  .table th { font-size: var(--font-size-xs); color: var(--color-text-muted); text-align: left; padding: var(--space-2) var(--space-3); border-bottom: 1px solid var(--color-border); }
  .table td { padding: var(--space-3); font-size: var(--font-size-sm); border-bottom: 1px solid var(--color-row-alt); }
  .income { color: var(--color-success); }
  .expense { color: var(--color-danger); }
</style>
```

`src/screens/Tasks.svelte`:
```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { listTasks, setTaskStatus, saveTask } from "../lib/api";
  import type { TasksData, TaskItem } from "../lib/types";

  let data: TasksData | null = $state(null);
  let showNew = $state(false);
  let newTitle = $state("");
  let newPriority = $state("Medium");

  onMount(async () => { data = await listTasks(); });

  async function toggleDone(task: TaskItem) {
    await setTaskStatus(task.id, task.done ? "Open" : "Done");
    data = await listTasks();
  }

  async function createTask() {
    if (!newTitle.trim()) return;
    await saveTask({ title: newTitle, description: "", priority: newPriority });
    newTitle = "";
    showNew = false;
    data = await listTasks();
  }

  const priorityLabel: Record<string, string> = { High: "Висока", Medium: "Середня", Low: "Низька" };
  const priorityClass: Record<string, string> = { High: "badge-red", Medium: "badge-orange", Low: "badge-gray" };
</script>

<div class="tasks">
  <header class="page-header">
    <h1>Задачі</h1>
    <button class="btn-primary" onclick={() => (showNew = !showNew)}>+ Нова</button>
  </header>

  {#if showNew}
    <div class="new-task-bar">
      <input bind:value={newTitle} placeholder="Назва задачі..." onkeydown={(e) => e.key === "Enter" && createTask()} />
      <select bind:value={newPriority}>
        <option value="High">Висока</option>
        <option value="Medium">Середня</option>
        <option value="Low">Низька</option>
      </select>
      <button class="btn-primary" onclick={createTask}>Додати</button>
    </div>
  {/if}

  {#if data}
    <div class="task-list">
      {#each data.open as task}
        <div class="task-item">
          <input type="checkbox" checked={task.done} onchange={() => toggleDone(task)} />
          <span class="task-title">{task.title}</span>
          <span class="badge {priorityClass[task.priority]}">{priorityLabel[task.priority]}</span>
          {#if task.due_date}<span class="due">{task.due_date}</span>{/if}
        </div>
      {/each}
      {#if data.done.length > 0}
        <div class="done-header">Виконано ({data.done.length})</div>
        {#each data.done as task}
          <div class="task-item done">
            <input type="checkbox" checked={task.done} onchange={() => toggleDone(task)} />
            <span class="task-title">{task.title}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .tasks { display: flex; flex-direction: column; height: 100%; }
  .page-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-5) var(--space-6); border-bottom: 1px solid var(--color-border); }
  h1 { font-size: var(--font-size-xl); font-weight: 600; }
  .new-task-bar { display: flex; gap: var(--space-2); padding: var(--space-3) var(--space-6); background: var(--color-surface); border-bottom: 1px solid var(--color-border); }
  .new-task-bar input { flex: 1; padding: 7px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .new-task-bar select { padding: 7px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .task-list { flex: 1; overflow-y: auto; padding: var(--space-3) var(--space-6); }
  .task-item { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3); border-bottom: 1px solid var(--color-row-alt); }
  .task-title { flex: 1; font-size: var(--font-size-sm); }
  .done .task-title { text-decoration: line-through; color: var(--color-text-muted); }
  .due { font-size: var(--font-size-xs); color: var(--color-text-muted); }
  .done-header { padding: var(--space-3); font-size: var(--font-size-xs); color: var(--color-text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-top: var(--space-4); }
</style>
```

`src/screens/Reports.svelte`:
```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { getReports } from "../lib/api";
  import KpiCard from "../lib/components/KpiCard.svelte";
  import type { ReportsData } from "../lib/types";

  let data: ReportsData | null = $state(null);
  const now = new Date();
  let year = $state(now.getFullYear());
  let month = $state(now.getMonth() + 1);

  onMount(load);

  async function load() {
    data = await getReports(year, month);
  }
</script>

<div class="reports">
  <header class="page-header">
    <h1>Звіти</h1>
    <div class="period-selector">
      <select bind:value={month} onchange={load}>
        {#each Array.from({length: 12}, (_, i) => i + 1) as m}
          <option value={m}>{m.toString().padStart(2, "0")}</option>
        {/each}
      </select>
      <select bind:value={year} onchange={load}>
        {#each [2024, 2025, 2026] as y}
          <option value={y}>{y}</option>
        {/each}
      </select>
    </div>
  </header>

  {#if data}
    <div class="content">
      <div class="kpi-strip">
        <KpiCard label="Дохід" value={data.metrics.revenue} />
        <KpiCard label="Витрати" value={data.metrics.expenses} />
        <KpiCard label="Прибуток" value={data.metrics.profit} />
      </div>

      {#if data.categories.length > 0}
        <section class="card">
          <h2>Витрати по статтях</h2>
          {#each data.categories as cat}
            <div class="category-row">
              <span class="cat-label">{cat.label}</span>
              <div class="cat-bar-wrap">
                <div class="cat-bar" style="width: {cat.percent}%"></div>
              </div>
              <span class="cat-amount">{cat.amount_str}</span>
              <span class="cat-pct">{cat.percent.toFixed(1)}%</span>
            </div>
          {/each}
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .reports { display: flex; flex-direction: column; height: 100%; }
  .page-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-5) var(--space-6); border-bottom: 1px solid var(--color-border); }
  h1 { font-size: var(--font-size-xl); font-weight: 600; }
  .period-selector { display: flex; gap: var(--space-2); }
  .period-selector select { padding: 6px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .content { padding: var(--space-6); }
  .kpi-strip { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-3); margin-bottom: var(--space-5); }
  .card { background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-lg); padding: var(--space-5); }
  h2 { font-size: var(--font-size-md); font-weight: 600; margin-bottom: var(--space-4); }
  .category-row { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-2) 0; }
  .cat-label { width: 180px; font-size: var(--font-size-sm); }
  .cat-bar-wrap { flex: 1; height: 6px; background: var(--color-row-alt); border-radius: 3px; }
  .cat-bar { height: 100%; background: var(--color-primary); border-radius: 3px; transition: width 0.3s; }
  .cat-amount { width: 100px; text-align: right; font-size: var(--font-size-sm); font-variant-numeric: tabular-nums; }
  .cat-pct { width: 50px; text-align: right; font-size: var(--font-size-xs); color: var(--color-text-muted); }
</style>
```

- [x] **Step 5: Перевірити всі три screens**

```bash
npm run dev
```

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/commands/ src/screens/Payments.svelte src/screens/Tasks.svelte src/screens/Reports.svelte
git commit -m "feat: implement Payments, Tasks, Reports commands and screens"
```

---

## Task 11: PDF Generation та File Dialogs

**Files:**
- Create: `src-tauri/src/commands/pdf.rs`
- Modify: `src-tauri/src/lib.rs` (додати команди)

- [x] **Step 1: Додати PDF команду**

```rust
// src-tauri/src/commands/pdf.rs
use crate::{pdf, state::AppState};
use tauri::{State, Manager};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

#[tauri::command]
pub async fn generate_pdf(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    doc_id: String,
    doc_kind: String,
) -> Result<String, String> {
    let uuid = Uuid::parse_str(&doc_id).map_err(|e| e.to_string())?;
    let company_id = state.company_id().await;

    // Генерація PDF — делегуємо до існуючого pdf/* модулю
    let pdf_bytes = pdf::generate_act(&state.pool, company_id, uuid)
        .await
        .map_err(|e| e.to_string())?;

    // Показати діалог збереження
    let path = app.dialog()
        .file()
        .set_file_name(&format!("{}.pdf", doc_id))
        .save_file()
        .await;

    if let Some(path) = path {
        tokio::fs::write(&path, pdf_bytes)
            .await
            .map_err(|e| e.to_string())?;
        Ok(path.to_string())
    } else {
        Err("Скасовано".to_string())
    }
}
```

- [x] **Step 2: Додати команду до `lib.rs` invoke_handler**

```rust
commands::pdf::generate_pdf,
```

- [x] **Step 3: Commit**

```bash
git add src-tauri/src/commands/pdf.rs src-tauri/src/lib.rs
git commit -m "feat: add PDF generation command with file save dialog"
```

---

## Task 12: BAS Import

**Files:**
- Create: `src-tauri/src/commands/import.rs`

- [x] **Step 1: Реалізувати `src-tauri/src/commands/import.rs`**

```rust
use crate::{import, state::AppState};
use tauri::{State, Manager};
use tauri_plugin_dialog::DialogExt;
use serde::Serialize;

#[derive(Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn import_bas(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    let paths = app.dialog()
        .file()
        .add_filter("BAS Export", &["xml", "xlsx"])
        .pick_files()
        .await;

    let Some(paths) = paths else {
        return Ok(ImportResult { imported: 0, errors: vec![] });
    };

    let company_id = state.company_id().await;
    let mut total = 0;
    let mut errors = vec![];

    for path in paths {
        match import::bas_contracts::import_file(&state.pool, company_id, path.as_ref()).await {
            Ok(n) => total += n,
            Err(e) => errors.push(e.to_string()),
        }
    }

    Ok(ImportResult { imported: total, errors })
}
```

- [x] **Step 2: Додати команду до `lib.rs`**

```rust
commands::import::import_bas,
```

- [x] **Step 3: Commit**

```bash
git add src-tauri/src/commands/import.rs src-tauri/src/lib.rs
git commit -m "feat: add BAS import command with file picker dialog"
```

---

## Task 13: Фінальне очищення

**Files:**
- Delete: `ui/` directory (Slint UI files)
- Delete: old `src/` directory (old Rust source — вже переміщено до src-tauri/)
- Modify: `CLAUDE.md`
- Modify: `.claude/lessons.md`

- [x] **Step 1: Перевірити що `src-tauri/` містить всі потрібні файли**

```bash
ls src-tauri/src/
# очікується: main.rs lib.rs state.rs commands/ db/ models/ import/ pdf/
```

- [x] **Step 2: Запустити повну збірку**

```bash
npm run build
```

Очікується: успішна збірка без помилок.

- [x] **Step 3: Видалити старі Slint файли**

```bash
rm -rf ui/
rm -rf src/bootstrap/ # старий bootstrap (якщо залишився)
```

> **Обережно:** `src/` тепер — Svelte фронтенд. Видаляти тільки `ui/` (Slint).

- [x] **Step 4: Оновити `CLAUDE.md`**

Замінити секцію стеку та структури проекту:
- Slint → Tauri + Svelte + TypeScript
- `ui/` → `src/` (Svelte)
- `src/ui/*.rs` → `src-tauri/src/commands/*.rs`
- `src/bootstrap.rs` → `src-tauri/src/lib.rs`

- [x] **Step 5: Перевірити кінцевий результат**

```bash
npm run dev
```

Всі 7 секцій відкриваються, дані завантажуються, форми зберігаються.

- [x] **Step 6: Final commit**

```bash
git add -A
git commit -m "feat: complete migration from Slint to Tauri + Svelte + TypeScript"
```

---

## Контрольний список перед завершенням

- [x] `cargo build` в `src-tauri/` проходить без помилок
- [x] `npm run build` проходить без помилок
- [x] Dashboard відображає KPI дані
- [x] Documents: 3 вкладки, пошук, статус badges
- [x] Counterparties: master-detail, форма редагування
- [x] Payments: список з KPI
- [x] Tasks: список з checkbox виконання
- [x] Reports: KPI + категорії
- [x] Settings: форма компанії зберігається
- [x] `ui/` директорія видалена
- [x] `CLAUDE.md` оновлений

---

## Примітки

**Що НЕ реалізовано в цьому плані (Phase 2):**
- Document editor (створення/редагування документу з позиціями) — найскладніший UI
- Bank CSV import UI
- Command palette
- Company switcher
- Keyboard shortcuts
- Dark mode toggle

Ці фічі додаються після успішної міграції базового функціоналу.

---

## Статус реалізації

✅ **Повністю реалізовано** — 2026-04-30

Міграція на Tauri 2 завершена. Slint повністю видалено.
