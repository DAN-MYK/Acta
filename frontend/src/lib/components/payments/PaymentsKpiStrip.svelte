<script lang="ts">
  import SkeletonCard from "../SkeletonCard.svelte";
  import type { PaymentsKpiDto } from "../../types";

  export let initialLoading = false;
  export let kpi: PaymentsKpiDto | null = null;
</script>

<div class="task-kpis" data-testid="payments-kpis">
  {#if initialLoading}
    <SkeletonCard count={4} />
  {:else}
    <div class="task-kpi-card">
      <strong>{kpi?.incomingStr ?? "0,00"}</strong>
      <span>{kpi?.incomingSub ?? "надходження"}</span>
    </div>
    <div class="task-kpi-card">
      <strong>{kpi?.outgoingStr ?? "0,00"}</strong>
      <span>{kpi?.outgoingSub ?? "витрати"}</span>
    </div>
    <div class="task-kpi-card">
      <strong>{kpi?.netStr ?? "0,00"}</strong>
      <span>Баланс</span>
    </div>
    <div class="task-kpi-card task-kpi-card-alert">
      <strong>{kpi?.unmatchedCount ?? 0}</strong>
      <span>Не зведено</span>
    </div>
  {/if}
</div>

<style>
  .task-kpis {
    display: grid;
    gap: 16px;
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .task-kpi-card {
    display: grid;
    gap: 10px;
    padding: 16px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 72%, var(--acta-color-bg-elevated));
  }

  .task-kpi-card strong {
    font-size: 1.35rem;
  }

  .task-kpi-card-alert {
    border-color: color-mix(in srgb, var(--acta-color-accent) 22%, var(--acta-color-border));
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--acta-color-accent-soft) 36%, transparent), transparent 80%),
      var(--acta-color-bg-elevated);
  }

  .task-kpi-card span {
    font-size: 12px;
    color: var(--acta-color-text-muted);
  }

  @media (max-width: 1080px) {
    .task-kpis {
      grid-template-columns: 1fr;
    }
  }
</style>
