<script lang="ts">
  import { PAYMENT_SCREEN_COPY } from "../../config/ui";

  export let importButton: HTMLButtonElement | null = null;
  export let loading = false;
  export let unmatchedCount = 0;
  export let busyImport = false;
  export let busyImportPick = false;
  export let busyImportCommit = false;
  export let busySync = false;
  export let onPickAndPreviewImport: () => void;
  export let onOpenEditor: () => void;
  export let onRunReconciliation: () => void;
  export let onImportCsv: () => void;
  export let onSyncBank: () => void;
  export let onOpenManualTemplate: () => void;
</script>

<div class="payments-toolbar" data-testid="payments-toolbar">
  <div class="payments-toolbar-main" data-testid="payments-toolbar-main">
    <button
      bind:this={importButton}
      class="btn-primary payments-toolbar-primary-action"
      on:click={onPickAndPreviewImport}
      disabled={busyImportPick || busyImport || busyImportCommit}
    >
      {busyImportPick ? PAYMENT_SCREEN_COPY.prepareImportPreview : PAYMENT_SCREEN_COPY.importStatement}
    </button>
    <button class="btn-secondary" on:click={onOpenEditor} disabled={loading}>
      Створити платіж
    </button>
    <button
      class="btn-secondary"
      on:click={onRunReconciliation}
      disabled={unmatchedCount === 0 || loading}
    >
      Запустити звірку{unmatchedCount > 0 ? ` (${unmatchedCount})` : ""}
    </button>
  </div>

  <div class="payments-toolbar-utility" data-testid="payments-toolbar-utility" aria-label="Додаткові дії">
    <button
      class="btn-ghost payments-toolbar-utility-action"
      on:click={onImportCsv}
      disabled={busyImport || busyImportPick || busyImportCommit}
    >
      {busyImport ? PAYMENT_SCREEN_COPY.importing : PAYMENT_SCREEN_COPY.importFromStorage}
    </button>
    <button
      class="btn-ghost payments-toolbar-utility-action"
      on:click={onSyncBank}
      disabled={busyImport || busySync || busyImportPick}
    >
      {busySync ? PAYMENT_SCREEN_COPY.syncing : PAYMENT_SCREEN_COPY.syncWithBank}
    </button>
    <button
      class="btn-ghost payments-toolbar-utility-action"
      on:click={onOpenManualTemplate}
      disabled={busyImport || busySync}
    >
      Шаблон CSV
    </button>
  </div>
</div>

<style>
  .payments-toolbar {
    display: grid;
    gap: 10px;
    margin-top: 18px;
  }

  .payments-toolbar-main,
  .payments-toolbar-utility {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }

  .payments-toolbar-utility {
    padding: 10px 12px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid color-mix(in srgb, var(--acta-color-border) 88%, transparent);
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 76%, var(--acta-color-bg-elevated));
  }

  .payments-toolbar-primary-action {
    min-width: min(100%, 240px);
  }

  .payments-toolbar-utility-action {
    min-height: 36px;
    padding: 0 12px;
    font-size: 0.92rem;
  }

  @media (max-width: 860px) {
    .payments-toolbar-main {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      align-items: stretch;
    }

    .payments-toolbar-primary-action {
      grid-column: 1 / -1;
      width: 100%;
      min-width: 0;
    }
  }

  @media (max-width: 720px) {
    .payments-toolbar {
      gap: 12px;
    }

    .payments-toolbar-main,
    .payments-toolbar-utility {
      width: 100%;
    }

    .payments-toolbar-main {
      gap: 8px;
    }

    .payments-toolbar-utility {
      gap: 8px;
      padding: 8px;
      justify-content: flex-start;
    }

    .payments-toolbar-utility-action {
      flex: 0 1 auto;
      min-height: 34px;
      padding: 0 10px;
      font-size: 0.88rem;
    }
  }

  @media (max-width: 560px) {
    .payments-toolbar-main {
      grid-template-columns: 1fr;
    }

    .payments-toolbar-primary-action {
      grid-column: auto;
    }
  }
</style>
