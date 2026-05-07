<script lang="ts">
  import SkeletonCard from "../components/SkeletonCard.svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import { dashboardStore } from "../stores/dashboard";
  import { documentsStore } from "../stores/documents";
  import { navigationStore } from "../stores/navigation";
  import { paymentsStore } from "../stores/payments";
  import { tasksStore } from "../stores/tasks";

  const dashboard = dashboardStore;
  const documents = documentsStore;
  const navigation = navigationStore;
  const payments = paymentsStore;
  const tasks = tasksStore;

  function openDashboardDocument(docId: string) {
    navigation.go("documents");
    void documents.open(docId);
  }

  function openDashboardTask(taskId: string) {
    navigation.go("tasks");
    void tasks.openEditor(taskId);
  }

  function openDashboardPayment(paymentId: string) {
    navigation.go("payments");
    void payments.openById(paymentId);
  }
</script>

<section class="dashboard-v1" data-testid="dashboard-screen">
  <div class="dashboard-header">
    <h2 class="sr-only">Дашборд</h2>
    <p>Операційна картина по активній компанії</p>
    <button class="btn-ghost" on:click={() => dashboard.load()} disabled={$dashboard.loading}>
      {$dashboard.loading ? "Оновлення..." : "Оновити"}
    </button>
  </div>

  {#if $dashboard.error}
    <p class="error">{$dashboard.error}</p>
  {/if}

  <div class="dashboard-kpis" data-testid="dashboard-kpis">
    {#if $dashboard.initialLoading}
      <SkeletonCard count={4} />
    {:else}
      {#each $dashboard.screen?.kpis ?? [] as kpi}
        <article class:positive={kpi.tone === "positive"} class:warning={kpi.tone === "warning"} class:danger={kpi.tone === "danger"} class="dashboard-kpi-card">
          <span>{kpi.label}</span>
          <strong>{kpi.value}</strong>
          <small>{kpi.detail}</small>
        </article>
      {/each}
    {/if}
  </div>

  <div class="dashboard-grid">
    <article class="dashboard-card wide" data-testid="dashboard-cashflow">
      <div class="card-title">
        <h3>Грошовий потік</h3>
        <span>Останні 90 днів</span>
      </div>
      {#if $dashboard.initialLoading}
        <SkeletonRow count={4} variant="compact" />
      {:else}
        <div class="cashflow-list">
          <div class="cashflow-row cashflow-head">
            <span class="cashflow-col-label">Місяць</span>
            <span class="cashflow-col-value">Нетто</span>
            <span class="cashflow-col-value">Надходження</span>
            <span class="cashflow-col-value">Витрати</span>
          </div>
          {#each $dashboard.screen?.cashflowRows ?? [] as row}
            <div class="cashflow-row">
              <strong>{row.label}</strong>
              <span class="cashflow-net">{row.netStr}</span>
              <span class="cashflow-income">{row.incomeStr}</span>
              <span class="cashflow-expense">{row.expenseStr}</span>
            </div>
          {/each}
        </div>
      {/if}
    </article>

    <article class="dashboard-card wide" data-testid="dashboard-recent-documents">
      <div class="card-title">
        <h3>Останні документи</h3>
        <button on:click={() => navigation.go("documents")}>Відкрити</button>
      </div>
      {#if $dashboard.initialLoading}
        <SkeletonRow count={3} />
      {:else}
        {#each $dashboard.screen?.recentDocuments ?? [] as doc}
          <button class="dashboard-list-row" on:click={() => openDashboardDocument(doc.id)}>
            <span>{doc.number} · {doc.counterparty}</span>
            <strong>{doc.amountStr}</strong>
          </button>
        {/each}
      {/if}
    </article>

    <article class="dashboard-card" data-testid="dashboard-upcoming-payments">
      <div class="card-title">
        <h3>Найближчі платежі</h3>
        <button on:click={() => navigation.go("payments")}>Відкрити</button>
      </div>
      {#if $dashboard.initialLoading}
        <SkeletonRow count={3} />
      {:else if ($dashboard.screen?.upcomingPayments?.length ?? 0) === 0}
        <p class="dashboard-list-empty">Очікуваних платежів поки немає.</p>
      {:else}
        {#each $dashboard.screen?.upcomingPayments ?? [] as payment}
          <button class:overdue={payment.isOverdue} class="dashboard-list-row" on:click={() => openDashboardPayment(payment.id)}>
            <span>{payment.contractor} · {payment.dateLabel}</span>
            <strong>{payment.amountStr}</strong>
          </button>
        {/each}
      {/if}
    </article>

    <article class="dashboard-card" data-testid="dashboard-urgent-tasks">
      <div class="card-title">
        <h3>Завдання у фокусі</h3>
        <button on:click={() => navigation.go("tasks")}>Відкрити</button>
      </div>
      {#if $dashboard.initialLoading}
        <SkeletonRow count={3} />
      {:else}
        {#each $dashboard.screen?.urgentTasks ?? [] as task}
          <button class="dashboard-list-row" on:click={() => openDashboardTask(task.id)}>
            <span>{task.title}</span>
            <strong>{task.dueDate || task.priorityLabel}</strong>
          </button>
        {/each}
      {/if}
    </article>
  </div>
</section>
