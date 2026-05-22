<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import AppIcon from "../AppIcon.svelte";
  import {
    DOCUMENT_DIRECTION_LABELS,
    DOCUMENTS_COPY,
    resolveDocumentKindMeta
  } from "../../config/ui";
  import { isFormattedMoneyNegative, parseMoneyToMinor } from "../../money";
  import type { DocumentsListDto } from "../../types";

  type DocumentListItem = DocumentsListDto["items"][number];
  type SortField = "number" | "counterparty" | "date" | "amount" | "kind" | "status" | "direction";
  type SortColumn = [SortField, string, string];

  const sortColumns: SortColumn[] = [
    ["number", "Номер", "Сортувати за номером"],
    ["counterparty", "Контрагент", "Сортувати за контрагентом"],
    ["date", "Дата", "Сортувати за датою"],
    ["amount", "Сума", "Сортувати за сумою"],
    ["kind", "Тип", "Сортувати за типом"],
    ["status", "Статус", "Сортувати за статусом"],
    ["direction", "Напрямок", "Сортувати за напрямком"]
  ];

  export let items: DocumentListItem[] = [];
  export let selectedIds: string[] = [];
  export let loading = false;

  const dispatch = createEventDispatcher<{
    open: string;
    toggleSelection: string;
    toggleSelectAll: void;
    bulkAdvanceStatus: void;
    bulkDelete: void;
  }>();

  let sortField: SortField | null = null;
  let sortDir: "asc" | "desc" = "asc";
  let pendingBulkDelete = false;

  $: sortedItems = (() => {
    if (!sortField) return items;

    const field = sortField;
    const direction = sortDir === "asc" ? 1 : -1;
    return [...items].sort((a, b) => {
      switch (field) {
        case "number":
          return direction * a.number.localeCompare(b.number, "uk", { numeric: true });
        case "counterparty":
          return direction * a.counterparty.localeCompare(b.counterparty, "uk");
        case "date":
          return direction * a.date.localeCompare(b.date);
        case "amount": {
          const av = parseMoneyToMinor(a.amountStr) ?? 0n;
          const bv = parseMoneyToMinor(b.amountStr) ?? 0n;
          return direction * (av < bv ? -1 : av > bv ? 1 : 0);
        }
        case "kind":
          return direction * a.kind.localeCompare(b.kind);
        case "status":
          return direction * a.status.localeCompare(b.status);
        case "direction":
          return direction * a.direction.localeCompare(b.direction);
        default:
          return 0;
      }
    });
  })();

  $: allSelected = items.length > 0 && items.every((item) => selectedIds.includes(item.id));

  $: if (pendingBulkDelete && (selectedIds.length === 0 || loading)) {
    pendingBulkDelete = false;
  }

  function toggleSort(field: SortField) {
    if (sortField === field) {
      if (sortDir === "asc") {
        sortDir = "desc";
      } else {
        sortField = null;
        sortDir = "asc";
      }
      return;
    }

    sortField = field;
    sortDir = "asc";
  }

  function sortOpacity(field: SortField, direction: "asc" | "desc") {
    if (sortField !== field) return 0.38;
    return sortDir === direction ? 1 : 0.22;
  }

  function getDocumentKindLabel(kind: string): string {
    return resolveDocumentKindMeta(kind).label;
  }

  function confirmBulkDelete() {
    if (selectedIds.length === 0 || loading) {
      pendingBulkDelete = false;
      return;
    }

    pendingBulkDelete = false;
    dispatch("bulkDelete");
  }
</script>

<div
  class="bulk-actions"
  class:bulk-actions-idle={selectedIds.length === 0}
  data-testid="documents-bulk-actions"
>
  <label class="bulk-select-all">
    <input
      type="checkbox"
      checked={allSelected}
      on:click|stopPropagation={() => dispatch("toggleSelectAll")}
    />
    <span>Вибрати все</span>
  </label>

  <button
    class="btn-secondary"
    disabled={selectedIds.length === 0 || loading}
    on:click={() => dispatch("bulkAdvanceStatus")}
  >
    Оновити статус вибраних
  </button>

  <button
    class="btn-danger"
    disabled={selectedIds.length === 0 || loading}
    on:click={() => { pendingBulkDelete = true; }}
  >
    Видалити вибрані
  </button>
</div>

<style>
  .doc-direction-badge {
    font-size: 11px;
    color: var(--acta-color-text-faint);
  }

  .doc-direction-badge[data-direction="outgoing"] {
    color: var(--acta-color-success);
  }

  .doc-direction-badge[data-direction="incoming"] {
    color: var(--acta-color-warning);
  }
</style>

{#if pendingBulkDelete}
  <div
    class="confirm-delete-banner"
    role="alertdialog"
    aria-live="assertive"
    aria-labelledby="documents-confirm-bulk-title"
    data-testid="documents-confirm-bulk-banner"
  >
    <div>
      <strong id="documents-confirm-bulk-title">Видалити вибрані?</strong>
      <p>{DOCUMENTS_COPY.confirmDeleteBulk}</p>
    </div>
    <div class="editor-dirty-actions">
      <button type="button" class="btn-ghost btn-sm" on:click={() => { pendingBulkDelete = false; }}>Скасувати</button>
      <button
        type="button"
        class="btn-danger btn-sm"
        on:click={confirmBulkDelete}
        disabled={selectedIds.length === 0 || loading}
        data-testid="documents-confirm-bulk-confirm"
      >
        Видалити
      </button>
    </div>
  </div>
{/if}

<div class="documents-table-card" data-testid="documents-list">
  <div class="documents-table-scroll">
    <div class="doc-trow doc-trow-head">
      <div></div>
      {#each sortColumns as column}
        <button
          class="doc-sort-btn"
          class:doc-sort-btn-right={column[0] === "amount"}
          type="button"
          on:click={() => toggleSort(column[0])}
          aria-label={column[2]}
          data-active={sortField === column[0] || null}
        >
          <span>{column[1]}</span>
          <svg width="10" height="12" viewBox="0 0 10 12" aria-hidden="true" fill="currentColor">
            <path d="M5 1.5L2 5h6L5 1.5Z" opacity={sortOpacity(column[0], "asc")}/>
            <path d="M5 10.5L8 7H2l3 3.5Z" opacity={sortOpacity(column[0], "desc")}/>
          </svg>
        </button>
      {/each}
    </div>

    {#each sortedItems as item (item.id)}
      <div class="doc-trow doc-trow-data" data-testid={`documents-row-${item.id}`}>
        <button
          class="doc-row-open"
          type="button"
          on:click={() => dispatch("open", item.id)}
          disabled={loading}
          aria-label={`Відкрити документ ${item.number}`}
        ></button>

        <label class="doc-row-checkbox doc-tcell" aria-label={`Вибрати ${item.number}`}>
          <input
            type="checkbox"
            checked={selectedIds.includes(item.id)}
            on:click|stopPropagation={() => dispatch("toggleSelection", item.id)}
          />
        </label>

        <span class="doc-tcell doc-tcell-number">
          <AppIcon name={resolveDocumentKindMeta(item.kind).icon} surface={true} size={16} />
          <span>{item.number}</span>
        </span>
        <span class="doc-tcell doc-tcell-counterparty">{item.counterparty}</span>
        <span class="doc-tcell doc-tcell-date">{item.date}</span>
        <span class="doc-tcell doc-tcell-amount money-value" data-negative={isFormattedMoneyNegative(item.amountStr)}>
          {item.amountStr}
        </span>
        <span class="doc-tcell doc-tcell-kind">
          <span class="doc-kind-badge">
            <AppIcon name={resolveDocumentKindMeta(item.kind).icon} size={14} />
            <span>{getDocumentKindLabel(item.kind)}</span>
          </span>
        </span>
        <span class="doc-tcell doc-tcell-status">
          <span class="doc-status-chip">{item.statusLabel}</span>
        </span>
        <span class="doc-tcell doc-tcell-direction">
          <span class="doc-direction-badge" data-direction={item.direction}>
            {DOCUMENT_DIRECTION_LABELS[item.direction] ?? item.direction}
          </span>
        </span>
      </div>
    {/each}
  </div>
</div>
