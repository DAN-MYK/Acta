<script lang="ts">
  export let tone: 'success' | 'warning' | 'danger' | 'info' | 'muted' = 'muted';
  export let dot: boolean = false;
  export let size: 'default' | 'lg' = 'default';
</script>

<span class="badge {tone} {size}">
  {#if dot}<span class="dot" aria-hidden="true"></span>{/if}
  <slot />
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: var(--acta-radius-sm);
    font-family: var(--acta-font-sans);
    font-weight: 500;
    white-space: nowrap;
  }

  .default {
    height: 20px;
    padding: 0 8px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .lg {
    height: 24px;
    padding: 0 10px;
    font-size: 12px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    flex-shrink: 0;
  }

  /* Tones */
  .success {
    background: var(--acta-color-success-soft);
    color: var(--acta-color-success);
  }

  .warning {
    background: var(--acta-color-warning-soft);
    color: var(--acta-color-warning);
  }

  .danger {
    background: var(--acta-color-danger-soft);
    color: var(--acta-color-danger);
  }

  .info {
    background: var(--acta-color-info-soft);
    color: var(--acta-color-info);
  }

  .muted {
    background: var(--acta-color-bg-subtle);
    color: var(--acta-color-text-muted);
  }
</style>
