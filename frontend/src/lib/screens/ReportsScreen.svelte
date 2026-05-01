<script lang="ts">
  import { reportsStore } from "../stores/reports";
  import type { PayableRowDto, ReceivableRowDto, ReportsScope, ReportsTab } from "../types";

  const reports = reportsStore;

  function onReportsSearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void reports.load({ query: input.value });
  }

  function onReportsTabChange(tab: ReportsTab) {
    void reports.load({ tab });
  }

  function onReportsScopeChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    void reports.load({ scope: select.value as ReportsScope });
  }

  function onReportsDateChange(field: "dateFrom" | "dateTo", event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void reports.load({ [field]: input.value });
  }

  function getReportHeadline(tab: ReportsTab | undefined): string {
    if (tab === "receivables") {
      return "Дебіторка під контролем";
    }
    if (tab === "payables") {
      return "Кредиторка без сюрпризів";
    }
    return "Контроль грошей і боргів";
  }

  function getReportHint(tab: ReportsTab | undefined): string {
    if (tab === "receivables") {
      return "Знайдіть прострочені надходження та документи, які потребують уваги першими.";
    }
    if (tab === "payables") {
      return "Перевіряйте, кому і коли потрібно платити, щоб не втрачати контроль над зобов'язаннями.";
    }
    return "Швидко оцініть рух грошей, дебіторку та кредиторку в одному місці.";
  }

  function overdueReceivables(rows: ReceivableRowDto[]): ReceivableRowDto[] {
    return rows.filter((row) => row.overdueDays > 0);
  }

  function overduePayables(rows: PayableRowDto[]): PayableRowDto[] {
    return rows.filter((row) => row.overdueDays > 0);
  }

  function getFocusTitle(tab: ReportsTab | undefined): string {
    if (tab === "receivables") {
      return "Потрібно сьогодні";
    }
    if (tab === "payables") {
      return "Найближчі виплати";
    }
    return "У фокусі зараз";
  }

  function getFocusDescription(tab: ReportsTab | undefined): string {
    if (tab === "receivables") {
      return "Зосередьтеся на прострочених надходженнях і найближчих очікуваних оплатах.";
    }
    if (tab === "payables") {
      return "Платежі, які не можна загубити між іншими операційними задачами.";
    }
    return "Коротка управлінська витяжка, щоб побачити ризики до занурення в таблицю.";
  }

  function getFocusValue(tab: ReportsTab | undefined): string {
    if (tab === "receivables") {
      return `${overdueReceivables($reports.screen?.receivablesRows ?? []).length}`;
    }
    if (tab === "payables") {
      return `${overduePayables($reports.screen?.payablesRows ?? []).length}`;
    }
    return $reports.screen?.summary.closingBalanceStr ?? "0,00 грн";
  }

  function getFocusMeta(tab: ReportsTab | undefined): string {
    if (tab === "receivables") {
      const first = overdueReceivables($reports.screen?.receivablesRows ?? [])[0];
      return first ? `${first.docNumber} · ${first.counterparty}` : "Прострочених надходжень немає";
    }
    if (tab === "payables") {
      const first = overduePayables($reports.screen?.payablesRows ?? [])[0];
      return first ? `${first.counterparty} · ${first.dueDate}` : "Прострочених виплат немає";
    }
    const first = $reports.screen?.bankRows?.[0];
    return first ? `${first.label} · ${first.netStr}` : "Дані по cashflow з'являться після вибору періоду";
  }

  function hasActiveRows(tab: ReportsTab | undefined): boolean {
    if (tab === "receivables") {
      return ($reports.screen?.receivablesRows?.length ?? 0) > 0;
    }
    if (tab === "payables") {
      return ($reports.screen?.payablesRows?.length ?? 0) > 0;
    }
    return ($reports.screen?.bankRows?.length ?? 0) > 0;
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Звіти</h2>
      <p>{getReportHeadline($reports.screen?.filter.tab)}</p>
    </div>
    <div class="panel-actions">
      <input
        placeholder="Пошук по поточному звіту"
        value={$reports.screen?.filter.query ?? ""}
        on:input={onReportsSearch}
      />
      <button class="btn-secondary" on:click={() => reports.exportCsv()}>Експортувати CSV</button>
    </div>
  </div>

  <div class="create-strip-card">
    <div class="create-strip-header">
      <div>
        <strong>Що аналізуємо</strong>
        <p>{getReportHint($reports.screen?.filter.tab)}</p>
      </div>
      <span class="doc-kind-badge">Звіт за сценарієм</span>
    </div>
  </div>

  <div class="reports-focus-grid">
    <div class="reports-focus-card">
      <span class="reports-focus-label">{getFocusTitle($reports.screen?.filter.tab)}</span>
      <strong>{getFocusValue($reports.screen?.filter.tab)}</strong>
      <p>{getFocusDescription($reports.screen?.filter.tab)}</p>
      <small>{getFocusMeta($reports.screen?.filter.tab)}</small>
    </div>
    <div class="reports-focus-card reports-focus-card-muted">
      <span class="reports-focus-label">Період аналізу</span>
      <strong>{$reports.screen?.filter.dateFrom ?? "—"} → {$reports.screen?.filter.dateTo ?? "—"}</strong>
      <p>Змініть період або scope, якщо потрібно порівняти компанії чи уточнити касовий сценарій.</p>
      <small>{$reports.screen?.filter.scope === "all" ? "Усі компанії" : "Активна компанія"}</small>
    </div>
  </div>

  <div class="reports-filters">
    <div class="task-tabs">
      <button class:active={$reports.screen?.filter.tab === "bank"} on:click={() => onReportsTabChange("bank")}>
        Рух грошей
      </button>
      <button
        class:active={$reports.screen?.filter.tab === "receivables"}
        on:click={() => onReportsTabChange("receivables")}
      >
        Нам мають
      </button>
      <button
        class:active={$reports.screen?.filter.tab === "payables"}
        on:click={() => onReportsTabChange("payables")}
      >
        Ми винні
      </button>
    </div>

    <div class="reports-filter-grid">
      <label>
        Показувати
        <select value={$reports.screen?.filter.scope ?? "active"} on:change={onReportsScopeChange}>
          <option value="active">Активна компанія</option>
          <option value="all">Усі компанії</option>
        </select>
      </label>
      <label>
        Дата від
        <input
          type="date"
          value={$reports.screen?.filter.dateFrom ?? ""}
          on:input={(event) => onReportsDateChange("dateFrom", event)}
        />
      </label>
      <label>
        Дата до
        <input
          type="date"
          value={$reports.screen?.filter.dateTo ?? ""}
          on:input={(event) => onReportsDateChange("dateTo", event)}
        />
      </label>
    </div>
  </div>

  <div class="reports-kpis">
    <div class="task-kpi-card">
      <strong>{$reports.screen?.summary.openingBalanceStr ?? "0,00 грн"}</strong>
      <span>{$reports.screen?.filter.tab === "bank" ? "Залишок на початок" : "База звіту"}</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$reports.screen?.summary.incomeStr ?? "0,00 грн"}</strong>
      <span>{$reports.screen?.filter.tab === "payables" ? "Очікувані надходження" : "Надходження"}</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$reports.screen?.summary.receivablesTotalStr ?? "0,00 грн"}</strong>
      <span>{$reports.screen?.filter.tab === "receivables" ? "До отримання" : "Дебіторка"}</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$reports.screen?.summary.payablesTotalStr ?? "0,00 грн"}</strong>
      <span>{$reports.screen?.filter.tab === "payables" ? "До оплати" : "Кредиторка"}</span>
    </div>
  </div>

  {#if $reports.message}
    <p class="message">{$reports.message}</p>
  {/if}

  {#if $reports.error}
    <p class="error">{$reports.error}</p>
  {/if}

  {#if !hasActiveRows($reports.screen?.filter.tab)}
    <div class="empty-state-card reports-empty-state">
      <strong>На цей період немає записів</strong>
      <p>Змініть період, scope або сценарій звіту, щоб знайти дані для аналізу.</p>
    </div>
  {:else if $reports.screen?.filter.tab === "bank"}
    <div class="reports-table reports-table-card">
      <div class="reports-table-row reports-table-head">
        <span>Група</span>
        <span>Надходження</span>
        <span>Витрати</span>
        <span>Чистий рух</span>
      </div>
      {#each $reports.screen?.bankRows ?? [] as row}
        <div class="reports-table-row">
          <span>{row.label}</span>
          <span>{row.incomeStr}</span>
          <span>{row.expenseStr}</span>
          <span>{row.netStr}</span>
        </div>
      {/each}
    </div>
  {:else if $reports.screen?.filter.tab === "receivables"}
    <div class="reports-table reports-table-card">
      <div class="reports-table-row reports-table-head reports-table-wide">
        <span>Документ</span>
        <span>Дата</span>
        <span>Компанія</span>
        <span>Контрагент</span>
        <span>Сума</span>
        <span>Очікувана дата</span>
        <span>Прострочка</span>
      </div>
      {#each $reports.screen?.receivablesRows ?? [] as row}
        <div class="reports-table-row reports-table-wide" class:reports-table-row-overdue={row.overdueDays > 0}>
          <span>{row.docNumber}</span>
          <span>{row.docDate}</span>
          <span>{row.companyName}</span>
          <span>{row.counterparty}</span>
          <span>{row.amountStr}</span>
          <span>{row.expectedDate || "—"}</span>
          <span>{row.overdueDays > 0 ? `Прострочено ${row.overdueDays} дн.` : "Без прострочки"}</span>
        </div>
      {/each}
    </div>
  {:else}
    <div class="reports-table reports-table-card">
      <div class="reports-table-row reports-table-head reports-table-wide">
        <span>Назва</span>
        <span>Компанія</span>
        <span>Контрагент</span>
        <span>Сума</span>
        <span>Дата</span>
        <span>Прострочка</span>
        <span>Повтор</span>
      </div>
      {#each $reports.screen?.payablesRows ?? [] as row}
        <div class="reports-table-row reports-table-wide" class:reports-table-row-overdue={row.overdueDays > 0}>
          <span>{row.title}</span>
          <span>{row.companyName}</span>
          <span>{row.counterparty || "—"}</span>
          <span>{row.amountStr}</span>
          <span>{row.dueDate}</span>
          <span>{row.overdueDays > 0 ? `Прострочено ${row.overdueDays} дн.` : "Без прострочки"}</span>
          <span>{row.recurrence}</span>
        </div>
      {/each}
    </div>
  {/if}
</section>
