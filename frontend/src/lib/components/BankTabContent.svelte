<script lang="ts">
  import PaymentGroups from "./payments/PaymentGroups.svelte";
  import PaymentImportPreviewPanel from "./payments/PaymentImportPreviewPanel.svelte";
  import PaymentMatchPreviewPanel from "./payments/PaymentMatchPreviewPanel.svelte";
  import PaymentsKpiStrip from "./payments/PaymentsKpiStrip.svelte";
  import PaymentsToolbar from "./payments/PaymentsToolbar.svelte";
  import {
    getManualPickerState,
    getPaymentBusyFlags,
    getPaymentFlowCopy,
    getPaymentGroups
  } from "./payments/payment-view-model";
  import { paymentsStore } from "../stores/payments";

  const payments = paymentsStore;
  let importButton: HTMLButtonElement | null = null;

  function onManualSearchInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    payments.updateManualMatchQuery(input.value);
  }

  function onSplitAllocationInput(documentId: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    payments.updateSplitAllocationAmount(documentId, input.value);
  }

  function focusImportButton() {
    importButton?.focus();
  }

  function runHeaderReconciliation() {
    const target = unmatchedPayments[0];
    if (target) {
      payments.reconcile(target.id);
    }
  }

  function openManualPickerForCurrentPreview() {
    const paymentId = $payments.matchPreview?.paymentId;
    if (paymentId) {
      payments.openManualMatchPicker(paymentId);
    }
  }

  $: items = $payments.list?.items ?? [];
  $: ({ unmatchedPayments, matchedPayments } = getPaymentGroups(items));
  $: ({ busyImport, busyImportPick, busyImportCommit, busySync } = getPaymentBusyFlags(
    $payments.loading,
    $payments.activeAction
  ));
  $: ({ canConfirm: manualPickerCanConfirm, disabledReason: manualPickerDisabledReason } =
    getManualPickerState($payments.manualPicker));
  $: flowCopy = getPaymentFlowCopy($payments.loading, $payments.activeAction);
  $: flowTitle = flowCopy?.title ?? null;
  $: flowDescription = flowCopy?.description ?? null;
</script>

<div class="bank-tab-root">
  <PaymentsToolbar
    bind:importButton
    loading={$payments.loading}
    unmatchedCount={unmatchedPayments.length}
    {busyImport}
    {busyImportPick}
    {busyImportCommit}
    {busySync}
    onPickAndPreviewImport={() => payments.pickAndPreviewImport()}
    onOpenEditor={() => payments.openEditor()}
    onRunReconciliation={runHeaderReconciliation}
    onImportCsv={() => payments.importCsv()}
    onSyncBank={() => payments.syncBank()}
    onOpenManualTemplate={() => payments.openManualTemplate()}
  />

  <PaymentsKpiStrip initialLoading={$payments.initialLoading} kpi={$payments.list?.kpi ?? null} />

  {#if flowTitle}
    <section class="flow-banner status-banner is-loading" data-testid="payments-flow-banner" role="status" aria-live="polite">
      <strong>{flowTitle}</strong>
      {#if flowDescription}
        <p>{flowDescription}</p>
      {/if}
    </section>
  {/if}

  {#if $payments.message && !$payments.loading}
    <div class="status-banner is-success" role="status" aria-live="polite">
      <div>
        <strong>Дію виконано</strong>
        <p>{$payments.message}</p>
      </div>
    </div>
  {/if}

  {#if $payments.error}
    <div class="status-banner is-error" role="alert" aria-live="assertive">
      <div>
        <strong>Потрібна увага</strong>
        <p>{$payments.error}</p>
      </div>
    </div>
  {/if}

  {#if $payments.importPreview}
    <PaymentImportPreviewPanel
      preview={$payments.importPreview}
      stale={$payments.importPreviewStale}
      {busyImportPick}
      {busyImportCommit}
      onRefresh={() => payments.refreshImportPreview()}
      onCommit={() => payments.commitImportPreview()}
      onCancel={() => payments.cancelImportPreview()}
    />
  {/if}

  {#if $payments.matchPreview}
    <PaymentMatchPreviewPanel
      preview={$payments.matchPreview}
      selectedCandidateId={$payments.selectedCandidateId}
      manualPicker={$payments.manualPicker}
      splitDraft={$payments.splitDraft}
      loading={$payments.loading}
      {manualPickerCanConfirm}
      {manualPickerDisabledReason}
      onConfirmAutoMatch={() => payments.confirmPreviewAutoMatch()}
      onConfirmSelectedCandidate={() => payments.confirmSelectedPreviewCandidate()}
      onOpenManualPicker={openManualPickerForCurrentPreview}
      onClosePreview={() => payments.closeMatchPreview()}
      onSelectPreviewCandidate={(documentId) => payments.selectPreviewCandidate(documentId)}
      onFocusImportButton={focusImportButton}
      onManualSearchInput={onManualSearchInput}
      onRefreshManualSearch={() => payments.searchManualMatchCandidates()}
      onAddManualCandidateToSplit={() => payments.addSelectedManualPickerCandidateToSplit()}
      onConfirmManualCandidate={() => payments.confirmManualPickerCandidate()}
      onCloseManualPicker={() => payments.closeManualMatchPicker()}
      onSelectManualCandidate={(documentId) => payments.selectManualPickerCandidate(documentId)}
      onSplitAllocationInput={onSplitAllocationInput}
      onRemoveSplitAllocation={(documentId) => payments.removeSplitAllocation(documentId)}
      onConfirmSplitDraft={() => payments.confirmSplitDraft()}
    />
  {/if}

  <PaymentGroups
    {unmatchedPayments}
    {matchedPayments}
    initialLoading={$payments.initialLoading}
    loading={$payments.loading}
    activePaymentId={$payments.activePaymentId}
    onOpenEditor={(payment) => payments.openEditor(payment)}
    onReconcile={(paymentId) => payments.reconcile(paymentId)}
    onUnreconcile={(paymentId) => payments.unreconcile(paymentId)}
  />
</div>

<style>
  .flow-banner {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-elevated);
  }

  .flow-banner {
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--acta-color-accent-soft) 32%, transparent), transparent 74%),
      var(--acta-color-bg-elevated);
  }

  .flow-banner p {
    margin: 6px 0 0;
    color: var(--acta-color-text-muted);
  }
</style>
