<script lang="ts" context="module">
  export type DocumentFiltersApplyDetail = {
    dateFrom: string | null;
    dateTo: string | null;
    statusFilter: string[];
    amountMin: string | null;
    amountMax: string | null;
    counterpartyFilterId: string | null;
  };
</script>

<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import {
    DOCUMENTS_FILTER_COPY,
    DOCUMENT_STATUS_OPTIONS
  } from "../../config/documents";

  type CounterpartyOption = {
    id: string;
    name: string;
  };

  export let open = false;
  export let loading = false;
  export let dateFrom: string | null = null;
  export let dateTo: string | null = null;
  export let statusFilter: string[] = [];
  export let amountMin: string | null = null;
  export let amountMax: string | null = null;
  export let overdueOnly = false;
  export let counterpartyFilterId: string | null = null;
  export let counterparties: CounterpartyOption[] = [];

  const dispatch = createEventDispatcher<{
    apply: DocumentFiltersApplyDetail;
    close: void;
  }>();

  let wasOpen = false;
  let panelDateFrom = "";
  let panelDateTo = "";
  let panelStatuses: string[] = [];
  let panelAmountMin = "";
  let panelAmountMax = "";
  let panelCounterpartyId = "";
  const AMOUNT_FILTER_PATTERN = /^\d+(?:\.\d+)?$/;

  $: if (open && !wasOpen) {
    syncDraftFromProps();
    wasOpen = true;
  } else if (!open && wasOpen) {
    wasOpen = false;
  }

  $: dateRangeError = panelDateFrom && panelDateTo && panelDateFrom > panelDateTo
    ? DOCUMENTS_FILTER_COPY.errors.dateRangeInvalid
    : null;

  $: amountRangeError = computeAmountError(panelAmountMin, panelAmountMax);

  function syncDraftFromProps() {
    panelDateFrom = dateFrom ?? "";
    panelDateTo = dateTo ?? "";
    panelStatuses = [...statusFilter];
    panelAmountMin = amountMin ?? "";
    panelAmountMax = amountMax ?? "";
    panelCounterpartyId = counterpartyFilterId ?? "";
  }

  function normalizeAmount(value: string): string | null {
    const normalized = value.trim().replace(/\s+/g, "").replace(",", ".");
    return normalized.length > 0 ? normalized : null;
  }

  function isValidAmount(value: string): boolean {
    return AMOUNT_FILTER_PATTERN.test(value);
  }

  function compareDecimalAmounts(left: string, right: string): number {
    const [leftIntRaw, leftFracRaw = ""] = left.split(".");
    const [rightIntRaw, rightFracRaw = ""] = right.split(".");
    const leftInt = leftIntRaw.replace(/^0+(?=\d)/, "");
    const rightInt = rightIntRaw.replace(/^0+(?=\d)/, "");

    if (leftInt.length !== rightInt.length) return leftInt.length > rightInt.length ? 1 : -1;
    if (leftInt !== rightInt) return leftInt > rightInt ? 1 : -1;

    const fractionLength = Math.max(leftFracRaw.length, rightFracRaw.length);
    const leftFrac = leftFracRaw.padEnd(fractionLength, "0");
    const rightFrac = rightFracRaw.padEnd(fractionLength, "0");
    if (leftFrac === rightFrac) return 0;
    return leftFrac > rightFrac ? 1 : -1;
  }

  function computeAmountError(minStr: string, maxStr: string): string | null {
    const minNormalized = normalizeAmount(minStr);
    const maxNormalized = normalizeAmount(maxStr);

    if (minNormalized && !isValidAmount(minNormalized)) return DOCUMENTS_FILTER_COPY.errors.amountInvalidFormat;
    if (maxNormalized && !isValidAmount(maxNormalized)) return DOCUMENTS_FILTER_COPY.errors.amountInvalidFormat;
    if (minNormalized !== null && maxNormalized !== null && compareDecimalAmounts(minNormalized, maxNormalized) > 0) {
      return DOCUMENTS_FILTER_COPY.errors.amountRangeInvalid;
    }

    return null;
  }

  function formatLocalDate(date: Date): string {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }

  function toggleStatus(code: string, checked: boolean) {
    panelStatuses = checked
      ? Array.from(new Set([...panelStatuses, code]))
      : panelStatuses.filter((status) => status !== code);
  }

  function onStatusChange(code: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    toggleStatus(code, input.checked);
  }

  function onDateSubpreset(kind: "today" | "week" | "month" | "quarter" | "year") {
    const today = new Date();

    if (kind === "today") {
      panelDateFrom = formatLocalDate(today);
      panelDateTo = formatLocalDate(today);
    } else if (kind === "week") {
      const start = new Date(today);
      start.setDate(today.getDate() - 6);
      panelDateFrom = formatLocalDate(start);
      panelDateTo = formatLocalDate(today);
    } else if (kind === "month") {
      const start = new Date(today.getFullYear(), today.getMonth(), 1);
      panelDateFrom = formatLocalDate(start);
      panelDateTo = formatLocalDate(today);
    } else if (kind === "quarter") {
      const quarter = Math.floor(today.getMonth() / 3);
      const start = new Date(today.getFullYear(), quarter * 3, 1);
      panelDateFrom = formatLocalDate(start);
      panelDateTo = formatLocalDate(today);
    } else {
      const start = new Date(today.getFullYear(), 0, 1);
      panelDateFrom = formatLocalDate(start);
      panelDateTo = formatLocalDate(today);
    }
  }

  function resetPanelDraft() {
    panelDateFrom = "";
    panelDateTo = "";
    panelStatuses = [];
    panelAmountMin = "";
    panelAmountMax = "";
    panelCounterpartyId = "";
  }

  function applyFilters() {
    dispatch("apply", {
      dateFrom: panelDateFrom || null,
      dateTo: panelDateTo || null,
      statusFilter: panelStatuses,
      amountMin: normalizeAmount(panelAmountMin),
      amountMax: normalizeAmount(panelAmountMax),
      counterpartyFilterId: panelCounterpartyId || null
    });
    dispatch("close");
  }

</script>

{#if open}
  <div
    id="documents-filter-popover"
    class="filter-popover"
    data-testid="documents-filter-panel"
    data-overdue-only={overdueOnly ? "true" : undefined}
    role="dialog"
    aria-label="Фільтр документів"
  >
    <fieldset class="filter-panel-section">
      <legend>{DOCUMENTS_FILTER_COPY.periodLabel}</legend>
      <div class="filter-panel-subpresets">
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset("today")}>
          {DOCUMENTS_FILTER_COPY.periodSubpresets.today}
        </button>
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset("week")}>
          {DOCUMENTS_FILTER_COPY.periodSubpresets.week}
        </button>
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset("month")}>
          {DOCUMENTS_FILTER_COPY.periodSubpresets.month}
        </button>
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset("quarter")}>
          {DOCUMENTS_FILTER_COPY.periodSubpresets.quarter}
        </button>
        <button type="button" class="kind-chip" on:click={() => onDateSubpreset("year")}>
          {DOCUMENTS_FILTER_COPY.periodSubpresets.year}
        </button>
      </div>
      <div class="filter-panel-grid-2">
        <label>
          <span>{DOCUMENTS_FILTER_COPY.periodFrom}</span>
          <input type="date" bind:value={panelDateFrom} disabled={loading} />
        </label>
        <label>
          <span>{DOCUMENTS_FILTER_COPY.periodTo}</span>
          <input type="date" bind:value={panelDateTo} disabled={loading} />
        </label>
      </div>
      {#if dateRangeError}
        <p class="filter-error" role="alert">{dateRangeError}</p>
      {/if}
    </fieldset>

    <fieldset class="filter-panel-section">
      <legend>{DOCUMENTS_FILTER_COPY.statusLabel}</legend>
      <div class="filter-panel-statuses">
        {#each DOCUMENT_STATUS_OPTIONS as option}
          <label class="status-checkbox">
            <input
              type="checkbox"
              value={option.value}
              checked={panelStatuses.includes(option.value)}
              disabled={loading}
              on:change={(event) => onStatusChange(option.value, event)}
            />
            {option.label}
          </label>
        {/each}
      </div>
    </fieldset>

    <fieldset class="filter-panel-section">
      <legend>{DOCUMENTS_FILTER_COPY.counterpartyLabel}</legend>
      <select
        bind:value={panelCounterpartyId}
        disabled={loading}
        data-testid="documents-counterparty-filter"
        aria-label="Фільтр за контрагентом"
      >
        <option value="">{DOCUMENTS_FILTER_COPY.counterpartyAll}</option>
        {#each counterparties as counterparty}
          <option value={counterparty.id}>{counterparty.name}</option>
        {/each}
      </select>
    </fieldset>

    <fieldset class="filter-panel-section">
      <legend>{DOCUMENTS_FILTER_COPY.amountLabel}</legend>
      <div class="filter-panel-grid-2">
        <label>
          <span>{DOCUMENTS_FILTER_COPY.amountFrom}</span>
          <input type="text" inputmode="decimal" bind:value={panelAmountMin} placeholder="0,00" disabled={loading} />
        </label>
        <label>
          <span>{DOCUMENTS_FILTER_COPY.amountTo}</span>
          <input type="text" inputmode="decimal" bind:value={panelAmountMax} placeholder="0,00" disabled={loading} />
        </label>
      </div>
      {#if amountRangeError}
        <p class="filter-error" role="alert">{amountRangeError}</p>
      {/if}
    </fieldset>

    <div class="documents-filter-actions">
      <button class="btn-ghost" type="button" on:click={resetPanelDraft} disabled={loading}>
        {DOCUMENTS_FILTER_COPY.reset}
      </button>
      <button
        class="btn-primary"
        type="button"
        data-testid="documents-filter-apply"
        on:click={applyFilters}
        disabled={loading || !!dateRangeError || !!amountRangeError}
      >
        {DOCUMENTS_FILTER_COPY.apply}
      </button>
    </div>
  </div>
{/if}
