<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let searchValue: string = '';
  export let filters: Array<{ id: string; label: string; count?: number; active?: boolean }> = [];
  export let savedViews: Array<{ id: string; label: string; count: number; active?: boolean }> = [];

  const dispatch = createEventDispatcher<{
    search: string;
    filterChange: string;
    viewChange: string;
  }>();

  function handleSearch(event: Event) {
    dispatch('search', (event.currentTarget as HTMLInputElement).value);
  }
</script>

<div class="commandbar">
  {#if savedViews.length > 0}
    <div class="views">
      {#each savedViews as v}
        <button
          class="view-pill"
          class:active={v.active}
          on:click={() => dispatch('viewChange', v.id)}
        >
          {v.label}
          <span class="view-count">{v.count}</span>
        </button>
      {/each}
    </div>
  {/if}

  <input
    class="commandbar-search acta-input"
    type="search"
    value={searchValue}
    placeholder="Пошук…"
    on:input={handleSearch}
    aria-label="Пошук"
  />

  {#each filters as f}
    <button
      class="filter-btn"
      class:active={f.active}
      on:click={() => dispatch('filterChange', f.id)}
    >
      {f.label}
      {#if f.active && f.count != null}
        <span class="filter-count">({f.count})</span>
      {/if}
    </button>
  {/each}

  <div class="commandbar-spacer"></div>

  <slot name="primary" />
</div>

<style>
  .commandbar {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    padding: 12px 0;
    flex-wrap: wrap;
  }

  .views {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .view-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: var(--acta-radius-pill);
    border: 1px solid transparent;
    font-family: var(--acta-font-sans);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    background: var(--acta-color-bg-subtle);
    color: var(--acta-color-text-muted);
    transition: background var(--acta-motion-fast), color var(--acta-motion-fast);
  }

  .view-pill:hover {
    background: var(--acta-color-bg-hover);
    color: var(--acta-color-text);
  }

  .view-pill.active {
    background: var(--acta-color-accent);
    color: #fff;
    border-color: transparent;
  }

  .view-count {
    font-family: var(--acta-font-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    opacity: 0.8;
  }

  .commandbar-search {
    width: 280px;
    flex-shrink: 0;
  }

  .commandbar-spacer {
    flex: 1;
  }

  .filter-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: var(--acta-density-button-h-sm);
    padding: 0 10px;
    border: 1px solid var(--acta-color-border-strong);
    border-radius: var(--acta-radius-md);
    background: var(--acta-color-bg-elevated);
    color: var(--acta-color-text);
    font-family: var(--acta-font-sans);
    font-size: 12px;
    cursor: pointer;
    transition: background var(--acta-motion-fast), border-color var(--acta-motion-fast);
  }

  .filter-btn:hover {
    background: var(--acta-color-bg-hover);
  }

  .filter-btn.active {
    background: var(--acta-color-accent-soft);
    border-color: var(--acta-color-accent);
    color: var(--acta-color-accent-text);
  }

  .filter-count {
    font-family: var(--acta-font-mono);
    font-size: 11px;
  }
</style>
