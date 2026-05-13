<script lang="ts">
  import type { TableColumn } from './table-types';

  export let columns: TableColumn[];
  export let rows: Record<string, unknown>[] = [];
  export let getRowId: (row: Record<string, unknown>) => string;
  export let selectedIds: string[] = [];
  export let onSelectChange: ((ids: string[]) => void) | undefined = undefined;
  export let onRowClick: ((row: Record<string, unknown>) => void) | undefined = undefined;
  export let sortBy: string | undefined = undefined;
  export let sortDir: 'asc' | 'desc' = 'asc';
  export let onSortChange: ((col: string, dir: 'asc' | 'desc') => void) | undefined = undefined;
  export let emptyTitle: string = 'Немає даних';
  export let emptySubtitle: string = '';

  function toggleSort(colId: string) {
    if (!onSortChange) return;
    const newDir: 'asc' | 'desc' = sortBy === colId && sortDir === 'asc' ? 'desc' : 'asc';
    onSortChange(colId, newDir);
  }

  function toggleSelectAll() {
    if (!onSelectChange) return;
    if (selectedIds.length === rows.length) {
      onSelectChange([]);
    } else {
      onSelectChange(rows.map(getRowId));
    }
  }

  function toggleRow(rowId: string) {
    if (!onSelectChange) return;
    if (selectedIds.includes(rowId)) {
      onSelectChange(selectedIds.filter(id => id !== rowId));
    } else {
      onSelectChange([...selectedIds, rowId]);
    }
  }

  function clearSelection() {
    onSelectChange?.([]);
  }

  $: allSelected = rows.length > 0 && selectedIds.length === rows.length;
  $: someSelected = selectedIds.length > 0;
</script>

{#if someSelected}
  <div class="bulk-banner" role="status">
    <span>Вибрано {selectedIds.length}</span>
    <slot name="bulk-actions" />
    <button class="bulk-clear" on:click={clearSelection} aria-label="Скасувати вибір">×</button>
  </div>
{/if}

<div class="table-wrapper">
  {#if rows.length === 0}
    <div class="empty-state">
      <div class="empty-icon" aria-hidden="true">
        <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
          <path d="M4 8h24M4 16h16M4 24h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </div>
      <p class="empty-title">{emptyTitle}</p>
      {#if emptySubtitle}<p class="empty-subtitle">{emptySubtitle}</p>{/if}
    </div>
  {:else}
    <table class="table">
      <thead>
        <tr>
          {#if onSelectChange}
            <th class="col-check">
              <input
                type="checkbox"
                checked={allSelected}
                indeterminate={someSelected && !allSelected}
                on:change={toggleSelectAll}
                aria-label="Вибрати всі рядки"
              />
            </th>
          {/if}
          {#each columns as col}
            <th
              class="th"
              class:sortable={col.sortable}
              class:sorted={sortBy === col.id}
              style:width={col.width}
              style:text-align={col.align ?? 'left'}
              aria-sort={col.sortable && sortBy === col.id ? (sortDir === 'asc' ? 'ascending' : 'descending') : undefined}
            >
              {#if col.sortable}
                <button
                  class="sort-btn"
                  on:click={() => toggleSort(col.id)}
                  aria-label="{col.header} — сортувати"
                >
                  {col.header}
                  <span class="sort-icon" aria-hidden="true">
                    {sortBy === col.id ? (sortDir === 'asc' ? '↑' : '↓') : '↕'}
                  </span>
                </button>
              {:else}
                {col.header}
              {/if}
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rows as row, i}
          {@const rowId = getRowId(row)}
          {@const isSelected = selectedIds.includes(rowId)}
          <tr
            class="tr"
            class:odd={i % 2 !== 0}
            class:selected={isSelected}
            class:clickable={!!onRowClick}
            tabindex={onRowClick ? 0 : undefined}
            role={onRowClick ? "button" : undefined}
            on:click={() => onRowClick?.(row)}
            on:keydown={(e) => { if (onRowClick && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); onRowClick(row); } }}
          >
            {#if onSelectChange}
              <td class="col-check" on:click|stopPropagation>
                <input
                  type="checkbox"
                  checked={isSelected}
                  on:change={() => toggleRow(rowId)}
                  aria-label="Вибрати рядок"
                />
              </td>
            {/if}
            {#each columns as col}
              <td
                class="td"
                class:align-right={col.align === 'right'}
                class:align-center={col.align === 'center'}
              >
                {col.accessor(row) ?? '—'}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .table-wrapper {
    width: 100%;
    overflow-x: auto;
  }

  .table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  .th {
    height: var(--acta-density-table-header-row);
    padding: 0 var(--acta-density-table-pad-x);
    background: var(--acta-color-bg-subtle);
    border-bottom: 1px solid var(--acta-color-border-strong);
    font-size: 11px;
    font-weight: 600;
    color: var(--acta-color-text-faint);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    text-align: left;
    white-space: nowrap;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .th.sortable {
    cursor: pointer;
    user-select: none;
  }

  .th.sortable:hover {
    color: var(--acta-color-text);
  }

  .sort-btn {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font: inherit;
    font-size: inherit;
    font-weight: inherit;
    color: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .sort-btn:focus-visible {
    outline: 2px solid var(--acta-color-accent);
    outline-offset: 1px;
    border-radius: 2px;
  }

  .sort-icon {
    color: var(--acta-color-text-faint);
    font-size: 10px;
  }

  .tr {
    height: var(--acta-density-table-row);
    background: var(--acta-color-bg-elevated);
    border-bottom: 1px solid var(--acta-color-border);
    transition: background var(--acta-motion-fast);
  }

  .tr.odd {
    background: var(--acta-color-bg-stripe);
  }

  .tr:hover {
    background: var(--acta-color-bg-hover);
  }

  .tr.selected {
    background: var(--acta-color-accent-soft);
  }

  .tr.clickable {
    cursor: pointer;
  }

  .td {
    padding: var(--acta-density-table-pad-y) var(--acta-density-table-pad-x);
    color: var(--acta-color-text);
    border-bottom: 1px solid var(--acta-color-border);
  }

  .td.align-right {
    text-align: right;
    font-family: var(--acta-font-mono);
    font-variant-numeric: tabular-nums;
  }

  .td.align-center {
    text-align: center;
  }

  .col-check {
    width: 40px;
    padding: 0 8px;
    text-align: center;
  }

  /* Bulk banner */
  .bulk-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 16px;
    background: var(--acta-color-accent-soft);
    color: var(--acta-color-accent-text);
    font-size: 13px;
    font-weight: 500;
    animation: slide-in var(--acta-motion-base);
  }

  .bulk-clear {
    margin-left: auto;
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--acta-color-accent-text);
    font-size: 18px;
    line-height: 1;
    padding: 0 4px;
  }

  /* Empty state */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 24px;
    gap: 8px;
  }

  .empty-icon {
    color: var(--acta-color-text-faint);
    margin-bottom: 8px;
  }

  .empty-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--acta-color-text);
  }

  .empty-subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--acta-color-text-muted);
    text-align: center;
  }

  @keyframes slide-in {
    from { transform: translateY(-100%); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
</style>
