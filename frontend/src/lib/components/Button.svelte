<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'ghost' | 'danger' = 'secondary';
  export let size: 'default' | 'sm' | 'icon' = 'default';
</script>

<button class="btn {variant} {size}" {...$$restProps}>
  <slot />
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border-radius: var(--acta-radius-md);
    font-family: var(--acta-font-sans);
    font-weight: 500;
    border: 1px solid transparent;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
    transition: background var(--acta-motion-fast), border-color var(--acta-motion-fast), filter var(--acta-motion-fast);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn:focus-visible {
    outline: 2px solid var(--acta-color-accent);
    outline-offset: 2px;
  }

  /* Sizes */
  .default {
    height: var(--acta-density-button-h);
    padding: 0 14px;
    font-size: 13px;
  }

  .sm {
    height: var(--acta-density-button-h-sm);
    padding: 0 10px;
    font-size: 12px;
    gap: 6px;
  }

  .icon {
    width: var(--acta-density-button-h);
    height: var(--acta-density-button-h);
    padding: 0;
    justify-content: center;
  }

  /* Variants */
  .primary {
    background: var(--acta-color-accent);
    color: #fff;
  }

  .primary:not(:disabled):hover {
    background: var(--acta-color-accent-hover);
  }

  .primary:not(:disabled):active {
    filter: brightness(0.94);
  }

  .secondary {
    background: var(--acta-color-bg-elevated);
    color: var(--acta-color-text);
    border-color: var(--acta-color-border-strong);
  }

  .secondary:not(:disabled):hover {
    background: var(--acta-color-bg-hover);
  }

  .secondary:not(:disabled):active {
    background: var(--acta-color-bg-subtle);
  }

  .ghost {
    background: transparent;
    color: var(--acta-color-text-muted);
  }

  .ghost:not(:disabled):hover {
    background: var(--acta-color-bg-hover);
    color: var(--acta-color-text);
  }

  .danger {
    background: var(--acta-color-danger);
    color: #fff;
  }

  .danger:not(:disabled):hover {
    filter: brightness(1.08);
  }
</style>
