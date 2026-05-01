<script lang="ts">
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import { reportsStore } from "../stores/reports";
  import type { PayableRowDto, ReceivableRowDto, ReportsScope, ReportsScreenDto, ReportsTab, TopCounterpartyRowDto } from "../types";

  interface ReportKpiCard {
    label: string;
    value: string;
    tone?: "default" | "accent" | "warning" | "danger";
  }

  const sortCollator = new Intl.Collator("uk", {
    numeric: true,
    sensitivity: "base"
  });

  const reports = reportsStore;
  let dateFromInput: HTMLInputElement | null = null;

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

  function focusReportsDateRange() {
    dateFromInput?.focus();
  }

  function getReportHeadline(tab: ReportsTab | undefined): string {
    if (tab === "pnl") {
      return "Дохід, витрати і результат";
    }
    if (tab === "receivables") {
      return "Нам мають заплатити";
    }
    if (tab === "payables") {
      return "Ми маємо заплатити";
    }
    return "Гроші на рахунках і в русі";
  }

  function getReportHint(tab: ReportsTab | undefined): string {
    if (tab === "pnl") {
      return "Показуємо, які категорії заробляють, де зростають витрати і який фінансовий результат ви отримали за період.";
    }
    if (tab === "receivables") {
      return "Швидко видно, хто вже затримує оплату, які документи потребують нагадування і де ризик касового розриву.";
    }
    if (tab === "payables") {
      return "Тут легко зрозуміти, кому платити першими, що вже прострочено і які регулярні виплати підходять.";
    }
    return "Огляд залишку, надходжень і виплат без переходу в окремі реєстри або Excel.";
  }

  function overdueReceivables(rows: ReceivableRowDto[]): ReceivableRowDto[] {
    return rows.filter((row) => row.overdueDays > 0);
  }

  function overduePayables(rows: PayableRowDto[]): PayableRowDto[] {
    return rows.filter((row) => row.overdueDays > 0);
  }

  function compareStrings(left: string, right: string): number {
    return sortCollator.compare(left || "", right || "");
  }

  function compareDates(left: string, right: string): number {
    return compareStrings(left || "9999-12-31", right || "9999-12-31");
  }

  function parseMoneyValue(value: string): number {
    const normalized = value.replace(/\s+/g, "").replace("грн", "").replace(",", ".").trim();
    const parsed = Number.parseFloat(normalized);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function stableSortRows<T>(rows: T[], compare: (left: T, right: T) => number): T[] {
    return rows
      .map((row, index) => ({ row, index }))
      .sort((left, right) => {
        const result = compare(left.row, right.row);
        return result !== 0 ? result : left.index - right.index;
      })
      .map(({ row }) => row);
  }

  function sortedReceivables(rows: ReceivableRowDto[]): ReceivableRowDto[] {
    return stableSortRows(rows, (left, right) => {
      if (left.overdueDays !== right.overdueDays) {
        return right.overdueDays - left.overdueDays;
      }

      const dueDateOrder = compareDates(left.expectedDate, right.expectedDate);
      if (dueDateOrder !== 0) {
        return dueDateOrder;
      }

      const amountOrder = parseMoneyValue(right.amountStr) - parseMoneyValue(left.amountStr);
      if (amountOrder !== 0) {
        return amountOrder;
      }

      return compareStrings(left.docNumber, right.docNumber);
    });
  }

  function sortedPayables(rows: PayableRowDto[]): PayableRowDto[] {
    return stableSortRows(rows, (left, right) => {
      if (left.overdueDays !== right.overdueDays) {
        return right.overdueDays - left.overdueDays;
      }

      const dueDateOrder = compareDates(left.dueDate, right.dueDate);
      if (dueDateOrder !== 0) {
        return dueDateOrder;
      }

      const amountOrder = parseMoneyValue(right.amountStr) - parseMoneyValue(left.amountStr);
      if (amountOrder !== 0) {
        return amountOrder;
      }

      return compareStrings(left.title, right.title);
    });
  }

  function daysUntil(dateValue: string): number | null {
    if (!dateValue) {
      return null;
    }

    const parsed = Date.parse(dateValue);
    if (Number.isNaN(parsed)) {
      return null;
    }

    return Math.ceil((parsed - Date.now()) / (24 * 60 * 60 * 1000));
  }

  function dueSoonReceivables(rows: ReceivableRowDto[]): number {
    return rows.filter((row) => {
      const days = daysUntil(row.expectedDate);
      return days !== null && days >= 0 && days <= 7;
    }).length;
  }

  function dueSoonPayables(rows: PayableRowDto[]): number {
    return rows.filter((row) => {
      const days = daysUntil(row.dueDate);
      return days !== null && days >= 0 && days <= 7;
    }).length;
  }

  function uniqueCounterpartiesCount(rows: Array<{ counterparty: string }>): number {
    return new Set(rows.map((row) => row.counterparty || "—")).size;
  }

  function getFocusTitle(tab: ReportsTab | undefined): string {
    if (tab === "pnl") {
      return "Фінансовий результат";
    }
    if (tab === "receivables" || tab === "payables") {
      return "Уваги сьогодні";
    }
    return "Ключовий фокус";
  }

  function getFocusDescription(tab: ReportsTab | undefined): string {
    if (tab === "pnl") {
      return "Орієнтир для керівника: чи перекриває дохід витрати і яка категорія найбільше впливає на результат.";
    }
    if (tab === "receivables") {
      return "Починайте з прострочених оплат, а далі переходьте до документів, де строк наближається протягом тижня.";
    }
    if (tab === "payables") {
      return "Першими перевіряйте прострочені та найближчі виплати, щоб не втратити контроль над календарем платежів.";
    }
    return "Звідси видно, як змінився залишок грошей і чи не з'явився ризик нестачі ліквідності.";
  }

  function getFocusValue(tab: ReportsTab | undefined): string {
    if (tab === "pnl") {
      return $reports.screen?.summary.pnlNetResultStr ?? "0,00 грн";
    }
    if (tab === "receivables") {
      return `${overdueReceivables($reports.screen?.receivablesRows ?? []).length}`;
    }
    if (tab === "payables") {
      return `${overduePayables($reports.screen?.payablesRows ?? []).length}`;
    }
    return $reports.screen?.summary.closingBalanceStr ?? "0,00 грн";
  }

  function getFocusMeta(tab: ReportsTab | undefined): string {
    if (tab === "pnl") {
      const first = $reports.screen?.pnlRows?.[0];
      return first ? `Найсильніше впливає: ${first.label} · ${first.netStr}` : "Категорії з'являться після вибору періоду.";
    }
    if (tab === "receivables") {
      const first = overdueReceivables(sortedReceivables($reports.screen?.receivablesRows ?? []))[0];
      return first
        ? `Перший у списку: ${first.docNumber} · ${first.counterparty}`
        : "Прострочених оплат зараз немає.";
    }
    if (tab === "payables") {
      const first = overduePayables(sortedPayables($reports.screen?.payablesRows ?? []))[0];
      return first ? `Перший ризик: ${first.counterparty || first.title} · ${first.dueDate}` : "Прострочених виплат зараз немає.";
    }
    const first = $reports.screen?.bankRows?.[0];
    return first ? `Найбільший рух: ${first.label} · ${first.netStr}` : "Дані з'являться після вибору періоду.";
  }

  function getKpiCards(tab: ReportsTab | undefined): ReportKpiCard[] {
    if (tab === "pnl") {
      return [
        {
          label: "Дохід за період",
          value: $reports.screen?.summary.pnlIncomeStr ?? "0,00 грн",
          tone: "accent"
        },
        {
          label: "Витрати за період",
          value: $reports.screen?.summary.pnlExpenseStr ?? "0,00 грн"
        },
        {
          label: "Фінансовий результат за період",
          value: $reports.screen?.summary.pnlNetResultStr ?? "0,00 грн",
          tone: "warning"
        },
        {
          label: "Категорій у звіті",
          value: `${$reports.screen?.pnlRows?.length ?? 0}`
        }
      ];
    }

    if (tab === "receivables") {
      const rows = sortedReceivables($reports.screen?.receivablesRows ?? []);
      return [
        {
          label: "Очікуємо отримати",
          value: $reports.screen?.summary.receivablesTotalStr ?? "0,00 грн",
          tone: "accent"
        },
        {
          label: "Прострочені оплати",
          value: `${overdueReceivables(rows).length}`,
          tone: overdueReceivables(rows).length > 0 ? "danger" : "default"
        },
        {
          label: "Оплати цього тижня",
          value: `${dueSoonReceivables(rows)}`,
          tone: dueSoonReceivables(rows) > 0 ? "warning" : "default"
        },
        {
          label: "Контрагентів у роботі",
          value: `${uniqueCounterpartiesCount(rows)}`
        }
      ];
    }

    if (tab === "payables") {
      const rows = sortedPayables($reports.screen?.payablesRows ?? []);
      return [
        {
          label: "Заплановано до оплати",
          value: $reports.screen?.summary.payablesTotalStr ?? "0,00 грн",
          tone: "accent"
        },
        {
          label: "Прострочені виплати",
          value: `${overduePayables(rows).length}`,
          tone: overduePayables(rows).length > 0 ? "danger" : "default"
        },
        {
          label: "Виплати цього тижня",
          value: `${dueSoonPayables(rows)}`,
          tone: dueSoonPayables(rows) > 0 ? "warning" : "default"
        },
        {
          label: "Регулярних платежів",
          value: `${rows.filter((row) => row.recurrence && row.recurrence !== "—").length}`
        }
      ];
    }

    return [
      {
        label: "Залишок на початок",
        value: $reports.screen?.summary.openingBalanceStr ?? "0,00 грн"
      },
      {
        label: "Надходження за період",
        value: $reports.screen?.summary.incomeStr ?? "0,00 грн",
        tone: "accent"
      },
      {
        label: "Виплати за період",
        value: $reports.screen?.summary.expenseStr ?? "0,00 грн"
      },
      {
        label: "Залишок на кінець",
        value: $reports.screen?.summary.closingBalanceStr ?? "0,00 грн",
        tone: "warning"
      }
    ];
  }

  function onToggleCounterparty(row: TopCounterpartyRowDto) {
    void reports.toggleCounterparty(row.counterpartyId);
  }

  function getTopCounterpartiesSubtitle(tab: string | undefined): string {
    if (tab === "receivables") return "Хто формує найбільшу дебіторку у вибраному періоді.";
    if (tab === "payables") return "Кому зараз найбільше винні або скоро маємо платити.";
    if (tab === "pnl") return "Хто найбільше впливає на фінрезультат за період.";
    return "По кому зараз проходить найбільший рух грошей.";
  }

  function getContextText(screen: ReportsScreenDto | null): string {
    const selected = screen?.selectedCounterparty;
    const tab = screen?.filter.tab;
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

  function hasActiveRows(tab: ReportsTab | undefined): boolean {
    if (tab === "pnl") {
      return ($reports.screen?.pnlRows?.length ?? 0) > 0;
    }
    if (tab === "receivables") {
      return ($reports.screen?.receivablesRows?.length ?? 0) > 0;
    }
    if (tab === "payables") {
      return ($reports.screen?.payablesRows?.length ?? 0) > 0;
    }
    return ($reports.screen?.bankRows?.length ?? 0) > 0;
  }
</script>

<section class="panel" data-testid="reports-screen">
  <div class="panel-header">
    <div>
      <h2>Звіти</h2>
      <p>{getReportHeadline($reports.screen?.filter.tab)}</p>
    </div>
    <div class="panel-actions">
      <button class="btn-primary" on:click={() => reports.exportExcelAndOpen()}>Відкрити Excel</button>
      <button class="btn-secondary" on:click={() => reports.exportExcel()}>Експортувати Excel</button>
      <input
        placeholder="Шукати документ, контрагента або категорію"
        value={$reports.screen?.filter.query ?? ""}
        on:input={onReportsSearch}
      />
      <button class="btn-secondary" on:click={() => reports.exportCsv()}>Експортувати CSV</button>
    </div>
  </div>

  <div class="create-strip-card">
    <div class="create-strip-header">
      <div>
        <strong>Що перевіряємо</strong>
        <p>{getReportHint($reports.screen?.filter.tab)}</p>
      </div>
      <span class="doc-kind-badge">Сценарний звіт</span>
    </div>
  </div>

  <div class="reports-focus-grid">
    <div class="reports-focus-card reports-focus-card-primary" data-testid="reports-focus-primary">
      <span class="reports-focus-label">{getFocusTitle($reports.screen?.filter.tab)}</span>
      <strong>{getFocusValue($reports.screen?.filter.tab)}</strong>
      <p>{getFocusDescription($reports.screen?.filter.tab)}</p>
      <small>{getFocusMeta($reports.screen?.filter.tab)}</small>
    </div>
    <div class="reports-focus-card reports-focus-card-muted">
      <span class="reports-focus-label">Параметри зрізу</span>
      <strong>{$reports.screen?.filter.dateFrom ?? "—"} → {$reports.screen?.filter.dateTo ?? "—"}</strong>
      <p>Змініть період або коло компаній, якщо хочете переглянути інший сценарій або уточнити причину відхилень.</p>
      <small>{$reports.screen?.filter.scope === "all" ? "Усі компанії" : "Активна компанія"}</small>
    </div>
  </div>

  <div class="reports-filters">
    <div class="task-tabs" role="tablist" aria-label="Режими звіту">
      <button
        class:active={$reports.screen?.filter.tab === "bank"}
        role="tab"
        aria-selected={$reports.screen?.filter.tab === "bank"}
        tabindex={$reports.screen?.filter.tab === "bank" ? 0 : -1}
        on:click={() => onReportsTabChange("bank")}
      >
        Гроші на рахунках і в русі
      </button>
      <button
        class:active={$reports.screen?.filter.tab === "pnl"}
        role="tab"
        aria-selected={$reports.screen?.filter.tab === "pnl"}
        tabindex={$reports.screen?.filter.tab === "pnl" ? 0 : -1}
        on:click={() => onReportsTabChange("pnl")}
      >
        Дохід, витрати і результат
      </button>
      <button
        class:active={$reports.screen?.filter.tab === "receivables"}
        role="tab"
        aria-selected={$reports.screen?.filter.tab === "receivables"}
        tabindex={$reports.screen?.filter.tab === "receivables" ? 0 : -1}
        on:click={() => onReportsTabChange("receivables")}
      >
        Нам мають заплатити
      </button>
      <button
        class:active={$reports.screen?.filter.tab === "payables"}
        role="tab"
        aria-selected={$reports.screen?.filter.tab === "payables"}
        tabindex={$reports.screen?.filter.tab === "payables" ? 0 : -1}
        on:click={() => onReportsTabChange("payables")}
      >
        Ми маємо заплатити
      </button>
    </div>

    <div class="reports-filter-grid">
      <label>
        Що показати у звіті
        <select value={$reports.screen?.filter.scope ?? "active"} on:change={onReportsScopeChange}>
          <option value="active">Лише активну компанію</option>
          <option value="all">Усі компанії</option>
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
  </div>

  <div class="reports-kpis">
    {#each getKpiCards($reports.screen?.filter.tab) as card}
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
        <p class="reports-top-counterparties-subtitle">{getTopCounterpartiesSubtitle($reports.screen?.filter.tab)}</p>
      </div>
      {#if $reports.screen?.selectedCounterparty}
        <div class="reports-top-counterparties-focus">
          <span>Фокус: {$reports.screen.selectedCounterparty.name}</span>
          <button
            class="btn-secondary reports-top-counterparties-reset"
            type="button"
            on:click={() => reports.load({ selectedCounterpartyId: null })}
          >
            Скинути
          </button>
        </div>
      {/if}
    </div>

    {#if ($reports.screen?.topCounterparties?.length ?? 0) === 0}
      <p class="reports-top-counterparties-empty">Контрагентів немає у вибраному діапазоні.</p>
    {:else}
      {#each $reports.screen?.topCounterparties ?? [] as row}
        <button
          class="reports-top-counterparty-row"
          class:active={$reports.screen?.filter.selectedCounterpartyId === row.counterpartyId}
          data-testid="top-counterparty-{row.counterpartyId}"
          on:click={() => onToggleCounterparty(row)}
          type="button"
        >
          <span class="reports-top-cp-name">{row.counterpartyName}</span>
          <span class="reports-top-cp-amount">{row.primaryAmountStr}</span>
          <span class="reports-top-cp-share">{row.sharePercent}%</span>
          <span class="reports-top-cp-secondary">{row.secondaryLabel}: {row.secondaryValue}</span>
          <div
            class="reports-top-counterparty-bar"
            role="progressbar"
            aria-valuenow={row.sharePercent}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-label="Частка {row.sharePercent}%"
          ><span style="width: {row.sharePercent}%"></span></div>
        </button>
      {/each}
    {/if}
  </div>

  <p class="reports-context-text">{getContextText($reports.screen)}</p>

  {#if $reports.message}
    <p class="message">{$reports.message}</p>
  {/if}

  {#if $reports.error}
    <p class="error">{$reports.error}</p>
  {/if}

  {#if $reports.initialLoading}
    <div class="reports-table-card" data-testid="reports-table-card">
      <SkeletonRow count={6} />
    </div>
  {:else if !hasActiveRows($reports.screen?.filter.tab)}
    <div class="empty-state-card reports-empty-state" data-testid="reports-empty-state">
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
          Р—РјС–РЅРёС‚Рё РїРµСЂС–РѕРґ
        </button>
      </div>
    </div>
  {:else if $reports.screen?.filter.tab === "bank"}
    <div class="reports-table-card" data-testid="reports-table-card">
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
              <span class="reports-cell-money">{row.incomeStr}</span>
              <span class="reports-cell-money">{row.expenseStr}</span>
              <span class="reports-cell-money">{row.netStr}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else if $reports.screen?.filter.tab === "pnl"}
    <div class="reports-table-card" data-testid="reports-table-card">
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
              <span class="reports-cell-money">{row.incomeStr}</span>
              <span class="reports-cell-money">{row.expenseStr}</span>
              <span class="reports-cell-money">{row.netStr}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else if $reports.screen?.filter.tab === "receivables"}
    <div class="reports-table-card" data-testid="reports-table-card">
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
          {#each sortedReceivables($reports.screen?.receivablesRows ?? []) as row}
            <div
              class="reports-table-row reports-table-row-receivables"
              class:reports-table-row-overdue={row.overdueDays > 0}
            >
              <span class="reports-cell-title">{row.docNumber}</span>
              <span class="reports-cell-date">{row.docDate}</span>
              <span class="reports-cell-company">{row.companyName}</span>
              <span class="reports-cell-company">{row.counterparty}</span>
              <span class="reports-cell-money">{row.amountStr}</span>
              <span class="reports-cell-date">{row.expectedDate || "—"}</span>
              <span class="reports-cell-status">
                {row.overdueDays > 0 ? `Прострочено ${row.overdueDays} дн.` : "Без прострочки"}
              </span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else}
    <div class="reports-table-card" data-testid="reports-table-card">
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
          {#each sortedPayables($reports.screen?.payablesRows ?? []) as row}
            <div
              class="reports-table-row reports-table-row-payables"
              class:reports-table-row-overdue={row.overdueDays > 0}
            >
              <span class="reports-cell-title">{row.title}</span>
              <span class="reports-cell-company">{row.companyName}</span>
              <span class="reports-cell-company">{row.counterparty || "—"}</span>
              <span class="reports-cell-money">{row.amountStr}</span>
              <span class="reports-cell-date">{row.dueDate}</span>
              <span class="reports-cell-status">
                {row.overdueDays > 0 ? `Прострочено ${row.overdueDays} дн.` : "Без прострочки"}
              </span>
              <span class="reports-cell-title">{row.recurrence}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</section>
