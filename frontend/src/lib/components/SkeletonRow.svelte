<script lang="ts">
  export let count = 5;
  export let variant: "default" | "compact" = "default";

  const widths = ["40%", "55%", "65%", "48%", "60%", "52%"];
</script>

{#each Array.from({ length: count }) as _, index}
  <div
    class:skeleton-row-compact={variant === "compact"}
    class="skeleton-row"
    data-testid="skeleton-row-item"
  >
    {#if variant === "default"}
      <div class="skeleton-icon sk" data-testid="skeleton-row-icon"></div>
    {/if}

    <div
      class="skeleton-copy"
      style={variant === "compact" ? "grid-column:1;min-width:0;" : undefined}
    >
      <div class="skeleton-line sk" style={`width:${widths[index % widths.length]}`}></div>
      <div
        class="skeleton-line skeleton-line-short sk"
        style={`width:${widths[(index + 2) % widths.length]}`}
      ></div>
    </div>

    <div class="skeleton-meta" style={variant === "compact" ? "grid-column:2;" : undefined}>
      <div class="skeleton-amount sk"></div>
      <div class="skeleton-badge sk"></div>
    </div>
  </div>
{/each}

<style>
  .skeleton-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    border-radius: var(--acta-radius-xl);
    background: color-mix(in srgb, var(--acta-color-bg-elevated) 88%, transparent);
    border: 1px solid var(--acta-color-border);
  }

  .skeleton-icon {
    width: 2.75rem;
    height: 2.75rem;
    border-radius: var(--acta-radius-lg);
    flex-shrink: 0;
  }

  .skeleton-copy {
    display: grid;
    gap: var(--space-2);
    min-width: 0;
  }

  .skeleton-row-compact {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .skeleton-line {
    height: 0.875rem;
  }

  .skeleton-line-short {
    height: 0.75rem;
  }

  .skeleton-meta {
    display: grid;
    justify-items: end;
    gap: var(--space-2);
  }

  .skeleton-amount {
    width: 5.5rem;
    height: 1rem;
  }

  .skeleton-badge {
    width: 3.5rem;
    height: 0.875rem;
    border-radius: 999px;
  }
</style>
