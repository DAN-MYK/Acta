# Reports Top Counterparties Live UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Додати на reports screen live-блок `Топ контрагентів`, який працює по активній вкладці і кліком перебудовує основну таблицю звіту по вибраному контрагенту.

**Architecture:** Зміна проходить через існуючий `reports_load` pipeline без нового endpoint. Ми розширюємо filter/screen DTO, додаємо backend loaders для top-counterparties по кожній вкладці, проводимо `selectedCounterpartyId` через store і Tauri API та добудовуємо `ReportsScreen.svelte` як операційний центр із drill-down і reset state.

**Tech Stack:** Rust, Tauri command DTOs, sqlx, chrono, rust_decimal, Svelte, TypeScript, Vitest, existing DB integration tests.

---

## File Structure

- Modify: `C:\Users\MykhailoDan\apps\Acta\src\models\reports.rs`
  Відповідальність: backend domain types для reports filter і top-counterparty aggregation rows.
- Modify: `C:\Users\MykhailoDan\apps\Acta\src\db\reports.rs`
  Відповідальність: SQL loaders для таблиць звіту та top-counterparties drill-down.
- Modify: `C:\Users\MykhailoDan\apps\Acta\src\tauri_api\reports.rs`
  Відповідальність: DTO contract, mapping у `ReportsScreenDto`, formatting, `reports_load`.
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\types.ts`
  Відповідальність: TS DTO types для нових filter fields і top-counterparties screen data.
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\api.ts`
  Відповідальність: передати `selectedCounterpartyId` у `reports_load`.
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\stores\reports.ts`
  Відповідальність: lifecycle reports filter, reset drill-down при зміні вкладки/фільтрів, helper actions.
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\screens\ReportsScreen.svelte`
  Відповідальність: live UI-картка `Топ контрагентів`, context text, polish controls.
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\screens\__tests__\ReportsScreen.test.ts`
  Відповідальність: UI rendering and interaction tests.
- Modify: `C:\Users\MykhailoDan\apps\Acta\tests\db_integration.rs`
  Відповідальність: backend aggregation/drill-down integration tests.

### Task 1: Backend Contracts For Drill-Down

**Files:**
- Modify: `C:\Users\MykhailoDan\apps\Acta\src\models\reports.rs`
- Modify: `C:\Users\MykhailoDan\apps\Acta\src\tauri_api\reports.rs`
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\types.ts`
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\api.ts`
- Test: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\screens\__tests__\ReportsScreen.test.ts`

- [ ] **Step 1: Write the failing contract test shape in the frontend screen test**

```ts
function makeReportsScreen(): ReportsScreenDto {
  return {
    filter: {
      tab: "bank",
      scope: "active",
      dateFrom: "2026-02-01",
      dateTo: "2026-05-01",
      query: "",
      selectedCounterpartyId: null
    },
    selectedCounterparty: null,
    topCounterparties: [
      {
        counterpartyId: "cp-1",
        counterpartyName: "ТОВ Ромашка",
        primaryAmountStr: "48 200,00 грн",
        secondaryLabel: "Чистий рух",
        secondaryValue: "29 200,00 грн",
        sharePercent: 100
      }
    ],
    summary: {
      openingBalanceStr: "125 000,00 грн",
      incomeStr: "48 200,00 грн",
      expenseStr: "19 000,00 грн",
      closingBalanceStr: "154 200,00 грн",
      receivablesTotalStr: "23 000,00 грн",
      payablesTotalStr: "14 500,00 грн",
      pnlIncomeStr: "62 000,00 грн",
      pnlExpenseStr: "21 400,00 грн",
      pnlNetResultStr: "40 600,00 грн"
    },
    bankRows: [],
    pnlRows: [],
    receivablesRows: [],
    payablesRows: []
  };
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:frontend -- frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
Expected: FAIL with TypeScript or runtime errors because `selectedCounterpartyId`, `selectedCounterparty`, and `topCounterparties` do not exist in current DTOs.

- [ ] **Step 3: Add minimal backend and frontend type contract**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReportsFilter {
    pub scope: ReportsScope,
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub query: String,
    pub selected_counterparty_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopCounterpartyRow {
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub primary_amount: Decimal,
    pub secondary_label: String,
    pub secondary_value: String,
    pub share_percent: u8,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsLoadRequest {
    pub tab: Option<String>,
    pub scope: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub query: Option<String>,
    pub selected_counterparty_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportsFilterDto {
    pub tab: String,
    pub scope: String,
    pub date_from: String,
    pub date_to: String,
    pub query: String,
    pub selected_counterparty_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectedCounterpartyDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopCounterpartyRowDto {
    pub counterparty_id: String,
    pub counterparty_name: String,
    pub primary_amount_str: String,
    pub secondary_label: String,
    pub secondary_value: String,
    pub share_percent: u8,
}
```

```ts
export interface ReportsFilterDto {
  tab: ReportsTab;
  scope: ReportsScope;
  dateFrom: string;
  dateTo: string;
  query: string;
  selectedCounterpartyId: string | null;
}

export interface SelectedCounterpartyDto {
  id: string;
  name: string;
}

export interface TopCounterpartyRowDto {
  counterpartyId: string;
  counterpartyName: string;
  primaryAmountStr: string;
  secondaryLabel: string;
  secondaryValue: string;
  sharePercent: number;
}

export interface ReportsScreenDto {
  filter: ReportsFilterDto;
  selectedCounterparty: SelectedCounterpartyDto | null;
  topCounterparties: TopCounterpartyRowDto[];
  summary: ReportsSummaryDto;
  bankRows: BankReportRowDto[];
  pnlRows?: BankReportRowDto[];
  receivablesRows: ReceivableRowDto[];
  payablesRows: PayableRowDto[];
}
```

```ts
export function reportsLoad(filter: ReportsFilterDto): Promise<ReportsScreenDto> {
  return appInvoke("reports_load", {
    request: {
      tab: filter.tab,
      scope: filter.scope,
      dateFrom: filter.dateFrom,
      dateTo: filter.dateTo,
      query: filter.query,
      selectedCounterpartyId: filter.selectedCounterpartyId
    }
  });
}
```

- [ ] **Step 4: Run targeted test and typecheck to verify the contract passes**

Run: `npm run test:frontend -- frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
Expected: PASS or move past DTO shape failures to the next missing UI behavior.

- [ ] **Step 5: Commit**

```bash
git add src/models/reports.rs src/tauri_api/reports.rs frontend/src/lib/types.ts frontend/src/lib/api.ts frontend/src/lib/screens/__tests__/ReportsScreen.test.ts
git commit -m "feat: extend reports contract for counterparty drill-down"
```

### Task 2: Backend Queries For Top Counterparties And Drill-Down

**Files:**
- Modify: `C:\Users\MykhailoDan\apps\Acta\src\db\reports.rs`
- Modify: `C:\Users\MykhailoDan\apps\Acta\src\tauri_api\reports.rs`
- Modify: `C:\Users\MykhailoDan\apps\Acta\tests\db_integration.rs`
- Test: `C:\Users\MykhailoDan\apps\Acta\tests\db_integration.rs`

- [ ] **Step 1: Write failing DB integration tests for top-counterparties and drill-down**

```rust
#[tokio::test]
async fn load_top_counterparties_bank_ranks_counterparties_by_flow() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_top_counterparties_bank;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let cp_a = create_test_counterparty(&pool, &suffix, &format!("ТОВ A {suffix}"), None, None).await?;
    let cp_b = create_test_counterparty(&pool, &suffix, &format!("ТОВ B {suffix}"), None, None).await?;

    let p1 = create_test_payment(&pool, DEFAULT_COMPANY_ID, Some(cp_a.id), dec!(10000), "income", today).await?;
    let p2 = create_test_payment(&pool, DEFAULT_COMPANY_ID, Some(cp_a.id), dec!(1000), "expense", today).await?;
    let p3 = create_test_payment(&pool, DEFAULT_COMPANY_ID, Some(cp_b.id), dec!(5000), "income", today).await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: String::new(),
        selected_counterparty_id: None,
    };

    let rows = load_top_counterparties_bank(&ctx, &filter).await?;

    assert_eq!(rows[0].counterparty_id, cp_a.id.to_string());
    assert_eq!(rows[0].counterparty_name, format!("ТОВ A {suffix}"));
    assert_eq!(rows[0].primary_amount, dec!(11000));
    assert_eq!(rows[0].share_percent, 100);
    assert_eq!(rows[1].counterparty_id, cp_b.id.to_string());

    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2, $3)").bind(p1).bind(p2).bind(p3).execute(&pool).await?;
    sqlx::query("DELETE FROM counterparties WHERE id IN ($1, $2)").bind(cp_a.id).bind(cp_b.id).execute(&pool).await?;
    Ok(())
}
```

```rust
#[tokio::test]
async fn load_bank_rows_respects_selected_counterparty_id() -> Result<()> {
    use acta::app_ctx::AppCtx;
    use acta::db::reports::load_bank_rows;
    use acta::models::reports::{ReportsScope, ResolvedReportsFilter};

    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let suffix = unique_suffix();
    let today = chrono::Utc::now().date_naive();
    let period_start = today - Duration::days(30);

    let cp_a = create_test_counterparty(&pool, &suffix, &format!("ТОВ Drill A {suffix}"), None, None).await?;
    let cp_b = create_test_counterparty(&pool, &suffix, &format!("ТОВ Drill B {suffix}"), None, None).await?;

    let p1 = create_test_payment(&pool, DEFAULT_COMPANY_ID, Some(cp_a.id), dec!(7000), "income", today).await?;
    let p2 = create_test_payment(&pool, DEFAULT_COMPANY_ID, Some(cp_b.id), dec!(3000), "income", today).await?;

    let ctx = AppCtx::new(pool.clone(), DEFAULT_COMPANY_ID);
    let filter = ResolvedReportsFilter {
        scope: ReportsScope::Active,
        date_from: period_start,
        date_to: today,
        query: String::new(),
        selected_counterparty_id: Some(cp_a.id.to_string()),
    };

    let rows = load_bank_rows(&ctx, &filter).await?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, cp_a.id.to_string());
    assert_eq!(rows[0].income, dec!(7000));

    sqlx::query("DELETE FROM payments WHERE id IN ($1, $2)").bind(p1).bind(p2).execute(&pool).await?;
    sqlx::query("DELETE FROM counterparties WHERE id IN ($1, $2)").bind(cp_a.id).bind(cp_b.id).execute(&pool).await?;
    Ok(())
}
```

- [ ] **Step 2: Run targeted backend tests to verify they fail**

Run: `cargo test load_top_counterparties_bank_ranks_counterparties_by_flow load_bank_rows_respects_selected_counterparty_id --test db_integration`
Expected: FAIL because `load_top_counterparties_bank` does not exist and loaders do not yet filter by `selected_counterparty_id`.

- [ ] **Step 3: Implement top-counterparty loaders and selected-counterparty filtering**

```rust
fn selected_counterparty_uuid(filter: &ResolvedReportsFilter) -> Option<uuid::Uuid> {
    filter
        .selected_counterparty_id
        .as_deref()
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
}
```

```rust
pub async fn load_top_counterparties_bank(
    ctx: &AppCtx,
    filter: &ResolvedReportsFilter,
) -> Result<Vec<TopCounterpartyRow>> {
    struct Row {
        counterparty_id: String,
        counterparty_name: String,
        income: Decimal,
        expense: Decimal,
    }

    let company_id = match filter.scope {
        ReportsScope::Active => Some(ctx.company_id()),
        ReportsScope::All => None,
    };

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            cp.id::text AS counterparty_id,
            cp.name AS counterparty_name,
            COALESCE(SUM(CASE WHEN p.direction = 'income' THEN p.amount ELSE 0 END), 0) AS income,
            COALESCE(SUM(CASE WHEN p.direction = 'expense' THEN p.amount ELSE 0 END), 0) AS expense
        FROM payments p
        JOIN counterparties cp ON cp.id = p.counterparty_id
        WHERE ($1::uuid IS NULL OR p.company_id = $1::uuid)
          AND p.date BETWEEN $2 AND $3
        GROUP BY cp.id, cp.name
        ORDER BY (COALESCE(SUM(CASE WHEN p.direction = 'income' THEN p.amount ELSE 0 END), 0)
                + COALESCE(SUM(CASE WHEN p.direction = 'expense' THEN p.amount ELSE 0 END), 0)) DESC,
                 cp.name ASC
        LIMIT 8
        "#,
    )
    .bind(company_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .fetch_all(ctx.pool())
    .await?;

    let max_primary = rows
        .iter()
        .map(|row| row.income + row.expense)
        .max()
        .unwrap_or(Decimal::ZERO);

    Ok(rows
        .into_iter()
        .map(|row| {
            let primary_amount = row.income + row.expense;
            let share_percent = if max_primary.is_zero() {
                0
            } else {
                (((primary_amount / max_primary) * Decimal::from(100u8)).round_dp(0))
                    .to_u8()
                    .unwrap_or(0)
            };

            TopCounterpartyRow {
                counterparty_id: row.counterparty_id,
                counterparty_name: row.counterparty_name,
                primary_amount,
                secondary_label: "Чистий рух".to_string(),
                secondary_value: format_money_ua(row.income - row.expense),
                share_percent,
            }
        })
        .collect())
}
```

```rust
WHERE p.company_id = $1
  AND p.date BETWEEN $2 AND $3
  AND ($4::uuid IS NULL OR p.counterparty_id = $4::uuid)
```

```rust
let selected_counterparty_id = selected_counterparty_uuid(filter);
// bind selected_counterparty_id into bank / receivables / payables / pnl loaders
```

```rust
let top_counterparties = match filter_dto.tab.as_str() {
    "receivables" => load_top_counterparties_receivables(ctx, &filter).await?,
    "payables" => load_top_counterparties_payables(ctx, &filter).await?,
    "pnl" => load_top_counterparties_pnl(ctx, &filter).await?,
    _ => load_top_counterparties_bank(ctx, &filter).await?,
};
```

- [ ] **Step 4: Run focused backend verification**

Run: `cargo test load_top_counterparties_bank_ranks_counterparties_by_flow load_bank_rows_respects_selected_counterparty_id --test db_integration`
Expected: PASS

Run: `cargo test reports --test db_integration`
Expected: PASS for existing report-related integration tests plus new top-counterparties coverage.

- [ ] **Step 5: Commit**

```bash
git add src/db/reports.rs src/tauri_api/reports.rs tests/db_integration.rs
git commit -m "feat: add reports top counterparties backend loaders"
```

### Task 3: Reports Store And Filter Lifecycle

**Files:**
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\stores\reports.ts`
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\api.ts`
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\types.ts`
- Test: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\screens\__tests__\ReportsScreen.test.ts`

- [ ] **Step 1: Write failing interaction tests for drill-down and reset**

```ts
it("loads drill-down when user selects a top counterparty", async () => {
  const { component, target } = renderReports();
  const button = target.querySelector('[data-testid="top-counterparty-cp-1"]') as HTMLButtonElement;

  button.click();

  expect(mocks.load).toHaveBeenCalledWith({ selectedCounterpartyId: "cp-1" });
  component.$destroy();
});

it("resets selectedCounterpartyId when tab changes", async () => {
  mocks.reportsState.set({
    screen: {
      ...makeReportsScreen(),
      filter: {
        ...makeReportsScreen().filter,
        selectedCounterpartyId: "cp-1"
      },
      selectedCounterparty: {
        id: "cp-1",
        name: "ТОВ Ромашка"
      }
    },
    loading: false,
    error: null,
    message: null
  });

  const { component, target } = renderReports();
  const pnlTab = Array.from(target.querySelectorAll("button")).find((button) =>
    button.textContent?.includes("P&L")
  ) as HTMLButtonElement;

  pnlTab.click();

  expect(mocks.load).toHaveBeenCalledWith({ tab: "pnl", selectedCounterpartyId: null });
  component.$destroy();
});
```

- [ ] **Step 2: Run frontend tests to verify they fail**

Run: `npm run test:frontend -- frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
Expected: FAIL because the screen/store do not yet emit `selectedCounterpartyId` transitions.

- [ ] **Step 3: Implement store lifecycle helpers**

```ts
const defaultFilter: ReportsFilterDto = {
  tab: "bank",
  scope: "active",
  dateFrom: defaultFrom,
  dateTo: defaultTo,
  query: "",
  selectedCounterpartyId: null
};
```

```ts
function shouldResetCounterparty(partial?: Partial<ReportsFilterDto>): boolean {
  return Boolean(
    partial &&
      ("tab" in partial ||
        "scope" in partial ||
        "dateFrom" in partial ||
        "dateTo" in partial ||
        "query" in partial)
  );
}
```

```ts
async load(partial?: Partial<ReportsFilterDto>) {
  const current = get({ subscribe }).screen?.filter ?? defaultFilter;
  const normalizedPartial =
    shouldResetCounterparty(partial) && !("selectedCounterpartyId" in (partial ?? {}))
      ? { ...partial, selectedCounterpartyId: null }
      : partial ?? {};

  const filter = {
    ...current,
    ...normalizedPartial
  };

  update((state) => ({ ...state, loading: true, error: null }));

  try {
    const screen = await reportsLoad(filter);
    update((state) => ({ ...state, screen, loading: false }));
  } catch (error) {
    update((state) => ({ ...state, loading: false, error: String(error) }));
  }
}
```

```ts
async toggleCounterparty(counterpartyId: string) {
  const currentId = get({ subscribe }).screen?.filter.selectedCounterpartyId ?? null;
  await this.load({
    selectedCounterpartyId: currentId === counterpartyId ? null : counterpartyId
  });
}
```

- [ ] **Step 4: Run frontend verification**

Run: `npm run test:frontend -- frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
Expected: PASS for drill-down lifecycle tests.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/stores/reports.ts frontend/src/lib/api.ts frontend/src/lib/types.ts frontend/src/lib/screens/__tests__/ReportsScreen.test.ts
git commit -m "feat: add reports counterparty drill-down state"
```

### Task 4: Live UI Block And Product Polish

**Files:**
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\screens\ReportsScreen.svelte`
- Modify: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\screens\__tests__\ReportsScreen.test.ts`

- [ ] **Step 1: Write failing UI assertions for top-counterparties card**

```ts
it("renders top counterparties card and active focus state", () => {
  mocks.reportsState.set({
    screen: {
      ...makeReportsScreen(),
      filter: {
        ...makeReportsScreen().filter,
        selectedCounterpartyId: "cp-1"
      },
      selectedCounterparty: {
        id: "cp-1",
        name: "ТОВ Ромашка"
      }
    },
    loading: false,
    error: null,
    message: null
  });

  const { component, target } = renderReports();

  expect(target.querySelector('[data-testid="reports-top-counterparties"]')).toBeTruthy();
  expect(target.textContent).toContain("Топ контрагентів");
  expect(target.textContent).toContain("Фокус: ТОВ Ромашка");
  expect(target.textContent).toContain("Скинути");

  component.$destroy();
});
```

```ts
it("renders context text for selected counterparty", () => {
  mocks.reportsState.set({
    screen: {
      ...makeReportsScreen(),
      filter: {
        ...makeReportsScreen().filter,
        tab: "receivables",
        selectedCounterpartyId: "cp-1"
      },
      selectedCounterparty: {
        id: "cp-1",
        name: "ТОВ Ромашка"
      }
    },
    loading: false,
    error: null,
    message: null
  });

  const { component, target } = renderReports();

  expect(target.textContent).toContain("Показано: дебіторка по контрагенту ТОВ Ромашка");
  component.$destroy();
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm run test:frontend -- frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
Expected: FAIL because the new card, focus state, and context text are not rendered yet.

- [ ] **Step 3: Implement the UI block and context microcopy**

```svelte
function getTopCounterpartiesSubtitle(tab: ReportsTab | undefined): string {
  if (tab === "receivables") return "Хто формує найбільшу дебіторку у вибраному періоді.";
  if (tab === "payables") return "Кому зараз найбільше винні або скоро маємо платити.";
  if (tab === "pnl") return "Хто найбільше впливає на фінрезультат за період.";
  return "По кому зараз проходить найбільший рух грошей.";
}

function getContextText(): string {
  const selected = $reports.screen?.selectedCounterparty;
  const tab = $reports.screen?.filter.tab;
  if (!selected) {
    if (tab === "receivables") return "Показано: загальна дебіторка по всіх контрагентах";
    if (tab === "payables") return "Показано: загальна кредиторка по всіх контрагентах";
    if (tab === "pnl") return "Показано: загальний фінрезультат по всіх контрагентах";
    return "Показано: загальний рух грошей по всіх контрагентах";
  }
  if (tab === "receivables") return `Показано: дебіторка по контрагенту ${selected.name}`;
  if (tab === "payables") return `Показано: кредиторка по контрагенту ${selected.name}`;
  if (tab === "pnl") return `Показано: фінрезультат по контрагенту ${selected.name}`;
  return `Показано: рух грошей по контрагенту ${selected.name}`;
}
```

```svelte
<div class="reports-focus-card reports-top-counterparties" data-testid="reports-top-counterparties">
  <div class="reports-top-counterparties-header">
    <div>
      <span class="reports-focus-label">Топ контрагентів</span>
      <p>{getTopCounterpartiesSubtitle($reports.screen?.filter.tab)}</p>
    </div>
    {#if $reports.screen?.selectedCounterparty}
      <button class="btn-ghost" on:click={() => reports.load({ selectedCounterpartyId: null })}>Скинути</button>
    {/if}
  </div>

  {#if $reports.screen?.selectedCounterparty}
    <p class="reports-top-counterparties-focus">
      Фокус: {$reports.screen.selectedCounterparty.name}
    </p>
  {/if}

  {#if ($reports.screen?.topCounterparties.length ?? 0) === 0}
    <p class="reports-top-counterparties-empty">Немає контрагентів для рейтингу в цьому періоді.</p>
  {:else}
    {#each $reports.screen?.topCounterparties ?? [] as row}
      <button
        class="reports-top-counterparty-row"
        class:active={$reports.screen?.filter.selectedCounterpartyId === row.counterpartyId}
        data-testid={`top-counterparty-${row.counterpartyId}`}
        on:click={() => reports.toggleCounterparty(row.counterpartyId)}
      >
        <div class="reports-top-counterparty-main">
          <strong>{row.counterpartyName}</strong>
          <span>{row.primaryAmountStr}</span>
        </div>
        <div class="reports-top-counterparty-meta">
          <span>{row.secondaryLabel}</span>
          <span>{row.secondaryValue}</span>
        </div>
        <div class="reports-top-counterparty-bar">
          <span style={`width: ${row.sharePercent}%`}></span>
        </div>
      </button>
    {/each}
  {/if}
</div>
```

```svelte
<div class="reports-table-context">
  <small>{getContextText()}</small>
</div>
```

- [ ] **Step 4: Run UI verification**

Run: `npm run test:frontend -- frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
Expected: PASS

Run: `npm run check`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/screens/ReportsScreen.svelte frontend/src/lib/screens/__tests__/ReportsScreen.test.ts
git commit -m "feat: add live top counterparties reports UI"
```

### Task 5: Full Verification Pass

**Files:**
- Modify: `C:\Users\MykhailoDan\apps\Acta\docs\superpowers\specs\2026-05-01-reports-top-counterparties-live-ui-design.md`
  only if implementation reveals a necessary clarification
- Test: `C:\Users\MykhailoDan\apps\Acta\frontend\src\lib\screens\__tests__\ReportsScreen.test.ts`
- Test: `C:\Users\MykhailoDan\apps\Acta\tests\db_integration.rs`

- [ ] **Step 1: Run backend report tests**

Run: `cargo test reports --test db_integration`
Expected: PASS with existing report coverage plus top-counterparty/drill-down assertions.

- [ ] **Step 2: Run frontend screen tests**

Run: `npm run test:frontend -- frontend/src/lib/screens/__tests__/ReportsScreen.test.ts`
Expected: PASS

- [ ] **Step 3: Run TypeScript verification**

Run: `npm run check`
Expected: PASS

- [ ] **Step 4: Manually sanity-check the final live scenario**

Run: `npm run tauri dev`
Expected: Reports screen opens, `Топ контрагентів` reacts per tab, active focus highlights, `Скинути` works, and table context text stays aligned with the selected counterparty.

- [ ] **Step 5: Commit finishing verification notes**

```bash
git add src/db/reports.rs src/tauri_api/reports.rs src/models/reports.rs frontend/src/lib/types.ts frontend/src/lib/api.ts frontend/src/lib/stores/reports.ts frontend/src/lib/screens/ReportsScreen.svelte frontend/src/lib/screens/__tests__/ReportsScreen.test.ts tests/db_integration.rs
git commit -m "test: verify live reports top counterparties flow"
```
