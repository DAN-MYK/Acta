<script lang="ts">
  export let count = 4;
</script>

<div class="skeleton-card-grid" data-testid="skeleton-card-grid">
  {#each Array.from({ length: count }) as _, index}
    <div aria-hidden="true" class="skeleton-card" data-testid="skeleton-card-item">
      <div class="skeleton-card-label sk" style={`width:${index % 2 === 0 ? "42%" : "55%"}`}></div>
      <div class="skeleton-card-value sk"></div>
      <div
        class="skeleton-card-subtitle sk"
        style={`width:${index % 2 === 0 ? "68%" : "58%"}`}
      ></div>
    </div>
  {/each}
</div>

<style>
  .skeleton-card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--space-4);
  }

  .skeleton-card {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-5);
    border-radius: var(--radius-2xl);
    background: color-mix(in srgb, var(--bg-card-strong) 90%, transparent);
    border: 1px solid var(--border-hairline);
    box-shadow: var(--floating-shadow);
  }

  .skeleton-card-label {
    height: 0.75rem;
  }

  .skeleton-card-value {
    width: 72%;
    height: 1.75rem;
  }

  .skeleton-card-subtitle {
    height: 0.875rem;
  }
</style>
