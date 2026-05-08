<script lang="ts">
  import Card from './Card.svelte';

  export let caption: string;
  export let value: number;
  export let currency: 'UAH' | 'count' = 'UAH';
  export let delta: number | undefined = undefined;
  export let direction: 'positive-up' | 'positive-down' = 'positive-up';
  export let context: string | undefined = undefined;

  $: tone = delta == null || delta === 0
    ? 'neutral'
    : delta > 0
      ? (direction === 'positive-up' ? 'good' : 'bad')
      : (direction === 'positive-up' ? 'bad' : 'good');

  function formatValue(val: number, curr: 'UAH' | 'count'): string {
    if (curr === 'count') return val.toLocaleString('uk-UA');
    return val.toLocaleString('uk-UA', { minimumFractionDigits: 0, maximumFractionDigits: 0 });
  }
</script>

<Card compact>
  <div class="kpi-caption">{caption}</div>
  <div class="kpi-row">
    <div class="kpi-value">
      {formatValue(value, currency)}
      {#if currency === 'UAH'}<span class="kpi-unit">грн</span>{/if}
    </div>
    {#if delta != null}
      <div class="kpi-delta {tone}">
        {#if delta > 0}
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M7 11V3M3 7l4-4 4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {:else if delta < 0}
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M7 3v8M11 7l-4 4-4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {:else}
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M3 7h8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        {/if}
        <span>{delta > 0 ? '+' : ''}{delta.toFixed(1)}%</span>
      </div>
    {/if}
  </div>
  {#if context != null}
    <div class="kpi-context">{context}</div>
  {/if}
</Card>

<style>
  .kpi-caption {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    color: var(--acta-color-text-faint);
    margin-bottom: 8px;
  }

  .kpi-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .kpi-value {
    font-family: var(--acta-font-mono);
    font-size: 28px;
    line-height: 32px;
    font-weight: 600;
    color: var(--acta-color-text);
    font-variant-numeric: tabular-nums;
  }

  .kpi-unit {
    font-size: 18px;
    font-weight: 400;
    color: var(--acta-color-text-muted);
    margin-left: 4px;
  }

  .kpi-delta {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-family: var(--acta-font-mono);
    font-size: 13px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }

  .kpi-delta.good {
    color: var(--acta-color-success);
  }

  .kpi-delta.bad {
    color: var(--acta-color-danger);
  }

  .kpi-delta.neutral {
    color: var(--acta-color-text-muted);
  }

  .kpi-context {
    margin-top: 6px;
    font-size: 12px;
    color: var(--acta-color-text-muted);
  }
</style>
