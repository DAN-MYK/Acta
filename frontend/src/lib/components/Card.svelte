<script lang="ts">
  export let compact: boolean = false;
</script>

<div class="card" class:compact>
  {#if $$slots.header}
    <header class="card-header"><slot name="header" /></header>
  {/if}
  <div class="card-body"><slot /></div>
  {#if $$slots.footer}
    <footer class="card-footer"><slot name="footer" /></footer>
  {/if}
</div>

<style>
  .card {
    background: var(--acta-color-bg-elevated);
    border: 1px solid var(--acta-color-border);
    border-radius: var(--acta-radius-xl);
    box-shadow: var(--acta-shadow-card);
    padding: 20px 24px;
  }

  .compact {
    padding: 16px 20px;
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding-bottom: 16px;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--acta-color-border);
  }


  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 16px;
    margin-top: 16px;
    border-top: 1px solid var(--acta-color-border);
    font-size: 13px;
    color: var(--acta-color-text-muted);
  }
</style>
