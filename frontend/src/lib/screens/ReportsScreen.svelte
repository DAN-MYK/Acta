<script lang="ts">
  import { reportsStore } from "../stores/reports";
  import type { ReportsScope, ReportsTab } from "../types";

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

  {#if $reports.screen?.filter.tab === "bank"}
    <div class="reports-table">
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
    <div class="reports-table">
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
    <div class="reports-table">
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
