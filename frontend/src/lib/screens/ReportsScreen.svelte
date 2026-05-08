<script lang="ts">
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import {
    formatOverdueDaysLabel,
    getReportActiveRowsCount,
    getReportContextText,
    getReportKpiCards,
    getReportTopCounterpartiesSubtitle,
    hasReportActiveRows,
    REPORT_SCOPE_OPTIONS,
    REPORT_TABS
  } from "../config/ui";
  import { isFormattedMoneyNegative } from "../money";
  import { formatDate } from "../date";
  import { counterpartiesStore } from "../stores/counterparties";
  import { navigationStore } from "../stores/navigation";
  import { reportsStore } from "../stores/reports";
  import type { ReportsScope, ReportsTab, TopCounterpartyRowDto } from "../types";

  const reports = reportsStore;
  const reportTabs = REPORT_TABS;
  const reportScopeOptions = REPORT_SCOPE_OPTIONS;
  const reportsTabPanelId = "reports-tabpanel";

  let dateFromInput: HTMLInputElement | null = null;
  let reportTabButtons: Array<HTMLButtonElement | null> = [];

  function isBankNameRow(row: TopCounterpartyRowDto): boolean {
    return row.counterpartyId.startsWith("bank-name:");
  }

  function isCounterpartyDrillable(row: TopCounterpartyRowDto, scope: ReportsScope | undefined): boolean {
    return !isBankNameRow(row) && scope !== "all";
  }

  function onReportsSearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void reports.load({ query: input.value });
  }

  function onReportsTabChange(tab: ReportsTab) {
    void reports.load({ tab });
  }

  function getTabId(tab: ReportsTab): string {
    return `reports-tab-${tab}`;
  }

  function focusReportTab(index: number) {
    const normalizedIndex = ((index % reportTabs.length) + reportTabs.length) % reportTabs.length;
    reportTabButtons[normalizedIndex]?.focus();
  }

  function handleReportTabKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      event.preventDefault();
      focusReportTab(index + 1);
      return;
    }

    if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      event.preventDefault();
      focusReportTab(index - 1);
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      focusReportTab(0);
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      focusReportTab(reportTabs.length - 1);
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onReportsTabChange(reportTabs[index].id);
      focusReportTab(index);
    }
  }

  function onReportsScopeChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    void reports.load({ scope: select.value as ReportsScope });
  }

  function onReportsDateChange(field: "dateFrom" | "dateTo", event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void reports.load({ [field]: input.value });
  }

  function focusReportsDateRange() {
    dateFromInput?.focus();
  }

  function onToggleCounterparty(row: TopCounterpartyRowDto) {
    void reports.toggleCounterparty(row.counterpartyId);
  }

  function onResetCounterpartyFocus() {
    const counterpartyId = $reports.screen?.selectedCounterparty?.id;
    if (!counterpartyId) {
      return;
    }

    void reports.toggleCounterparty(counterpartyId);
  }

  async function onOpenCounterpartyCard(row: TopCounterpartyRowDto) {
    if (!isCounterpartyDrillable(row, $reports.screen?.filter.scope)) {
      return;
    }

    await counterpartiesStore.load();
    await counterpartiesStore.open(row.counterpartyId);
    navigationStore.go("counterparties");
  }

  function onResetAllFilters() {
    void reports.resetFilters();
  }
</script>

<section
  class="panel"
  data-testid="reports-screen"
  aria-busy={$reports.loading && !$reports.initialLoading ? "true" : undefined}
>
  <div class="reports-toolbar">
    <button class="btn-secondary" on:click={() => reports.exportExcelAndOpen()} disabled={$reports.loading}>
      Відкрити Excel
    </button>
    <button class="btn-ghost" on:click={() => reports.exportExcel()} disabled={$reports.loading}>
      Експортувати Excel
    </button>
    <button class="btn-ghost" on:click={() => reports.exportCsv()} disabled={$reports.loading}>
      Експортувати CSV
    </button>
  </div>

  <div class="reports-filters">
    <div class="task-tabs" role="tablist" aria-label="Режими звіту">
      {#each reportTabs as tab, index}
        <button
          bind:this={reportTabButtons[index]}
          class:active={$reports.screen?.filter.tab === tab.id}
          id={getTabId(tab.id)}
          role="tab"
          aria-selected={$reports.screen?.filter.tab === tab.id}
          aria-controls={reportsTabPanelId}
          tabindex={$reports.screen?.filter.tab === tab.id ? 0 : -1}
          on:click={() => onReportsTabChange(tab.id)}
          on:keydown={(event) => handleReportTabKeydown(event, index)}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div class="reports-filter-grid">
      <label>
        Що показати у звіті
        <select value={$reports.screen?.filter.scope ?? "active"} on:change={onReportsScopeChange}>
          {#each reportScopeOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label>
        Період від
        <input
          bind:this={dateFromInput}
          type="date"
          value={$reports.screen?.filter.dateFrom ?? ""}
          on:input={(event) => onReportsDateChange("dateFrom", event)}
        />
      </label>
      <label>
        Період до
        <input
          type="date"
          value={$reports.screen?.filter.dateTo ?? ""}
          on:input={(event) => onReportsDateChange("dateTo", event)}
        />
      </label>
    </div>

    <label class="reports-search-row">
      <span>Пошук у звіті</span>
      <input
        type="search"
        placeholder="Шукати документ, контрагента або категорію"
        value={$reports.screen?.filter.query ?? ""}
        on:input={onReportsSearch}
      />
    </label>
  </div>

  <div class="reports-kpis" data-testid="reports-focus-primary">
    {#each getReportKpiCards($reports.screen, $reports.screen?.filter.tab) as card}
      <div class="task-kpi-card reports-kpi-card" data-tone={card.tone ?? "default"}>
        <strong>{card.value}</strong>
        <span>{card.label}</span>
      </div>
    {/each}
  </div>

  <div class="reports-top-counterparties" data-testid="reports-top-counterparties">
    <div class="reports-top-counterparties-header">
      <div>
        <span class="reports-focus-label">Топ контрагентів</span>
        <p class="reports-top-counterparties-subtitle">
          {getReportTopCounterpartiesSubtitle($reports.screen?.filter.tab)}
        </p>
      </div>
      {#if $reports.screen?.selectedCounterparty}
        <div class="reports-top-counterparties-focus" data-testid="reports-top-counterparties-focus">
          <span class="reports-top-counterparties-focus-name">
            Фокус: {$reports.screen.selectedCounterparty.name}
          </span>
          <span class="reports-top-counterparties-focus-meta">
            У таблиці нижче: {getReportActiveRowsCount($reports.screen)}
          </span>
          <button class="btn-secondary reports-top-counterparties-reset" type="button" on:click={onResetCounterpartyFocus}>
            Скинути фокус
          </button>
        </div>
      {/if}
    </div>

    {#if $reports.initialLoading}
      <div data-testid="reports-top-counterparties-skeleton">
        <SkeletonRow count={3} />
      </div>
    {:else if ($reports.screen?.topCounterparties?.length ?? 0) === 0}
      <p class="reports-top-counterparties-empty">Контрагентів немає у вибраному діапазоні.</p>
    {:else}
      <ol class="reports-top-counterparty-list">
        {#each $reports.screen?.topCounterparties ?? [] as row, index}
          {@const isActive = $reports.screen?.filter.selectedCounterpartyId === row.counterpartyId}
          {@const drillable = isCounterpartyDrillable(row, $reports.screen?.filter.scope)}
          <li class="reports-top-counterparty-item">
            <button
              class="reports-top-counterparty-row"
              class:active={isActive}
              data-testid="top-counterparty-{row.counterpartyId}"
              on:click={() => onToggleCounterparty(row)}
              type="button"
              aria-pressed={isActive}
            >
              <span class="reports-top-cp-rank" aria-hidden="true">{index + 1}</span>
              <span class="reports-top-cp-name">
                {row.counterpartyName}
                {#if isBankNameRow(row)}
                  <span class="reports-top-cp-tag">без картки</span>
                {/if}
              </span>
              <span class="reports-top-cp-amount money-value" data-negative={isFormattedMoneyNegative(row.primaryAmountStr)}>
                {row.primaryAmountStr}
              </span>
              <span class="reports-top-cp-share">{row.sharePercent}%</span>
              <span class="reports-top-cp-secondary">{row.secondaryLabel}: {row.secondaryValue}</span>
              <div
                class="reports-top-counterparty-bar"
                role="progressbar"
                aria-valuenow={row.sharePercent}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuetext="{row.sharePercent}% від лідера"
                aria-label="Частка {row.sharePercent}%"
              ><span style="width: {row.sharePercent}%"></span></div>
            </button>
            {#if isActive && drillable}
              <button
                class="btn-ghost reports-top-counterparty-cta"
                type="button"
                data-testid="top-counterparty-open-{row.counterpartyId}"
                on:click={() => onOpenCounterpartyCard(row)}
              >
                Відкрити картку контрагента
              </button>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
  </div>

  <p class="reports-context-text">{getReportContextText($reports.screen)}</p>

  {#if $reports.message}
    <div class="status-banner is-success" role="status" aria-live="polite">
      <div>
        <strong>Дію виконано</strong>
        <p>{$reports.message}</p>
      </div>
    </div>
  {/if}

  {#if $reports.error}
    <div class="status-banner is-error" role="alert" aria-live="assertive">
      <div>
        <strong>Потрібна увага</strong>
        <p>{$reports.error}</p>
      </div>
    </div>
  {/if}

  {#if $reports.initialLoading}
    <div
      class="reports-table-card"
      data-testid="reports-table-card"
      id={reportsTabPanelId}
      role="tabpanel"
      aria-labelledby={getTabId($reports.screen?.filter.tab ?? "bank")}
    >
      <SkeletonRow count={6} />
    </div>
  {:else if !hasReportActiveRows($reports.screen, $reports.screen?.filter.tab)}
    <div
      class="empty-state-card reports-empty-state"
      data-testid="reports-empty-state"
      id={reportsTabPanelId}
      role="tabpanel"
      aria-labelledby={getTabId($reports.screen?.filter.tab ?? "bank")}
    >
      <span class="empty-state-eyebrow">Уточніть зріз</span>
      <strong>На цей період немає записів</strong>
      <p>Змініть період, коло компаній або сценарій звіту, щоб знайти дані для аналізу.</p>
      <div class="empty-state-actions">
        <button
          class="btn-secondary"
          type="button"
          data-testid="reports-empty-primary-action"
          on:click={focusReportsDateRange}
        >
          Змінити період
        </button>
        <button
          class="btn-ghost"
          type="button"
          data-testid="reports-empty-reset-action"
          on:click={onResetAllFilters}
        >
          Скинути фільтри
        </button>
      </div>
    </div>
  {:else if $reports.screen?.filter.tab === "bank"}
    <div
      class="reports-table-card"
      data-testid="reports-table-card"
      id={reportsTabPanelId}
      role="tabpanel"
      aria-labelledby={getTabId("bank")}
    >
      <div class="reports-table-scroll">
        <div class="reports-table">
          <div class="reports-table-row reports-table-row-head reports-table-row-bank">
            <span class="reports-cell-title">Група руху</span>
            <span class="reports-cell-money">Надходження</span>
            <span class="reports-cell-money">Виплати</span>
            <span class="reports-cell-money">Чистий рух</span>
          </div>
          {#each $reports.screen?.bankRows ?? [] as row}
            <div class="reports-table-row reports-table-row-bank">
              <span class="reports-cell-title">{row.label}</span>
              <span class="reports-cell-money money-value" data-negative={isFormattedMoneyNegative(row.incomeStr)}>{row.incomeStr}</span>
              <span class="reports-cell-money money-value" data-negative={isFormattedMoneyNegative(row.expenseStr)}>{row.expenseStr}</span>
              <span class="reports-cell-money money-value" data-negative={isFormattedMoneyNegative(row.netStr)}>{row.netStr}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else if $reports.screen?.filter.tab === "pnl"}
    <div
      class="reports-table-card"
      data-testid="reports-table-card"
      id={reportsTabPanelId}
      role="tabpanel"
      aria-labelledby={getTabId("pnl")}
    >
      <div class="reports-table-scroll">
        <div class="reports-table">
          <div class="reports-table-row reports-table-row-head reports-table-row-bank">
            <span class="reports-cell-title">Категорія</span>
            <span class="reports-cell-money">Дохід</span>
            <span class="reports-cell-money">Витрати</span>
            <span class="reports-cell-money">Результат</span>
          </div>
          {#each $reports.screen?.pnlRows ?? [] as row}
            <div class="reports-table-row reports-table-row-bank">
              <span class="reports-cell-title">{row.label}</span>
              <span class="reports-cell-money money-value" data-negative={isFormattedMoneyNegative(row.incomeStr)}>{row.incomeStr}</span>
              <span class="reports-cell-money money-value" data-negative={isFormattedMoneyNegative(row.expenseStr)}>{row.expenseStr}</span>
              <span class="reports-cell-money money-value" data-negative={isFormattedMoneyNegative(row.netStr)}>{row.netStr}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else if $reports.screen?.filter.tab === "receivables"}
    <div
      class="reports-table-card"
      data-testid="reports-table-card"
      id={reportsTabPanelId}
      role="tabpanel"
      aria-labelledby={getTabId("receivables")}
    >
      <div class="reports-table-scroll">
        <div class="reports-table">
          <div class="reports-table-row reports-table-row-head reports-table-row-receivables">
            <span class="reports-cell-title">Документ</span>
            <span class="reports-cell-date">Дата документа</span>
            <span class="reports-cell-company">Компанія</span>
            <span class="reports-cell-company">Контрагент</span>
            <span class="reports-cell-money">Сума до отримання</span>
            <span class="reports-cell-date">Очікувана оплата</span>
            <span class="reports-cell-status">Статус строку</span>
          </div>
          {#each $reports.screen?.receivablesRows ?? [] as row}
            <div
              class="reports-table-row reports-table-row-receivables"
              class:reports-table-row-overdue={row.overdueDays > 0}
            >
              <span class="reports-cell-title">{row.docNumber}</span>
              <span class="reports-cell-date">{formatDate(row.docDate)}</span>
              <span class="reports-cell-company">{row.companyName}</span>
              <span class="reports-cell-company">{row.counterparty}</span>
              <span class="reports-cell-money money-value" data-negative={isFormattedMoneyNegative(row.amountStr)}>{row.amountStr}</span>
              <span class="reports-cell-date">{formatDate(row.expectedDate)}</span>
              <span class="reports-cell-status">
                {formatOverdueDaysLabel(row.overdueDays)}
              </span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else}
    <div
      class="reports-table-card"
      data-testid="reports-table-card"
      id={reportsTabPanelId}
      role="tabpanel"
      aria-labelledby={getTabId("payables")}
    >
      <div class="reports-table-scroll">
        <div class="reports-table">
          <div class="reports-table-row reports-table-row-head reports-table-row-payables">
            <span class="reports-cell-title">Платіж</span>
            <span class="reports-cell-company">Компанія</span>
            <span class="reports-cell-company">Контрагент</span>
            <span class="reports-cell-money">Сума до оплати</span>
            <span class="reports-cell-date">Крайній строк</span>
            <span class="reports-cell-status">Статус строку</span>
            <span class="reports-cell-title">Повторюваність</span>
          </div>
          {#each $reports.screen?.payablesRows ?? [] as row}
            <div
              class="reports-table-row reports-table-row-payables"
              class:reports-table-row-overdue={row.overdueDays > 0}
            >
              <span class="reports-cell-title">{row.title}</span>
              <span class="reports-cell-company">{row.companyName}</span>
              <span class="reports-cell-company">{row.counterparty || "—"}</span>
              <span class="reports-cell-money money-value" data-negative={isFormattedMoneyNegative(row.amountStr)}>{row.amountStr}</span>
              <span class="reports-cell-date">{formatDate(row.dueDate)}</span>
              <span class="reports-cell-status">
                {formatOverdueDaysLabel(row.overdueDays)}
              </span>
              <span class="reports-cell-title">{row.recurrence}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</section>
