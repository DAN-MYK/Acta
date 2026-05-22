<script lang="ts">
  import { PAYMENT_SCREEN_COPY } from "../../config/ui";
  import {
    getPaymentCandidateHint,
    getPaymentDocumentKindLabel,
    getPaymentPreviewCopy
  } from "../../paymentsPresentation";
  import type { PaymentMatchPreviewDto } from "../../types";
  import PaymentManualMatchPanel from "./PaymentManualMatchPanel.svelte";
  import PaymentSplitDraftPanel from "./PaymentSplitDraftPanel.svelte";
  import type {
    PaymentManualPickerViewState,
    PaymentSplitDraftViewState
  } from "./payment-view-model";

  export let preview: PaymentMatchPreviewDto;
  export let selectedCandidateId: string | null = null;
  export let manualPicker: PaymentManualPickerViewState | null = null;
  export let splitDraft: PaymentSplitDraftViewState | null = null;
  export let loading = false;
  export let manualPickerCanConfirm = false;
  export let manualPickerDisabledReason = "";
  export let onConfirmAutoMatch: () => void;
  export let onConfirmSelectedCandidate: () => void;
  export let onOpenManualPicker: () => void;
  export let onClosePreview: () => void;
  export let onSelectPreviewCandidate: (documentId: string) => void;
  export let onFocusImportButton: () => void;
  export let onManualSearchInput: (event: Event) => void;
  export let onRefreshManualSearch: () => void;
  export let onAddManualCandidateToSplit: () => void;
  export let onConfirmManualCandidate: () => void;
  export let onCloseManualPicker: () => void;
  export let onSelectManualCandidate: (documentId: string) => void;
  export let onSplitAllocationInput: (documentId: string, event: Event) => void;
  export let onRemoveSplitAllocation: (documentId: string) => void;
  export let onConfirmSplitDraft: () => void;

  $: previewCopy = getPaymentPreviewCopy(preview);
</script>

<section class="chain-panel">
  <div class="chain-panel-header">
    <div>
      <strong>{previewCopy?.title ?? ""}</strong>
      <p>{previewCopy?.description ?? ""}</p>
    </div>
    <div class="editor-actions">
      {#if preview.decisionKind === "exact" && preview.autoMatch}
        <button
          class="btn-primary"
          on:click={onConfirmAutoMatch}
          disabled={loading}
        >
          {PAYMENT_SCREEN_COPY.confirmAutoMatch}
        </button>
      {:else if preview.decisionKind === "ambiguous"}
        <button
          class="btn-primary"
          on:click={onConfirmSelectedCandidate}
          disabled={loading}
        >
          {PAYMENT_SCREEN_COPY.confirmPreviewCandidate}
        </button>
        <button class="btn-secondary" on:click={onOpenManualPicker} disabled={loading}>
          {PAYMENT_SCREEN_COPY.chooseAnotherDocument}
        </button>
      {/if}
      <button class="btn-ghost" on:click={onClosePreview} disabled={loading}>
        {PAYMENT_SCREEN_COPY.closePreview}
      </button>
    </div>
  </div>

  {#if preview.decisionKind === "exact" && preview.autoMatch}
    <div class="doc-row payment-row payment-row-matched">
      <div class="task-row-main">
        <div>
          <strong>{preview.autoMatch.title}</strong>
          <p>{getPaymentDocumentKindLabel(preview.autoMatch.documentKind)} • {preview.autoMatch.amountStr}</p>
        </div>
        <div class="task-row-meta">
          <span class="task-pill">Рекомендація</span>
          <span>Автопідтвердження доступне</span>
        </div>
      </div>
    </div>
  {:else if preview.decisionKind === "ambiguous"}
    <div class="documents-list">
      {#each preview.candidates as candidate}
        <div
          class="doc-row payment-row"
          class:payment-row-matched={selectedCandidateId === candidate.documentId}
        >
          <div class="task-row-main">
            <div>
              <strong>{candidate.title}</strong>
              <p>{getPaymentDocumentKindLabel(candidate.documentKind)} • {candidate.openAmountStr}</p>
              <p>{getPaymentCandidateHint(candidate)}</p>
            </div>
            <div class="task-row-meta">
              <span class="task-pill">Скоринг {candidate.totalScore.toFixed(2)}</span>
              {#if selectedCandidateId === candidate.documentId}
                <span class="payment-state payment-state-matched">Обраний варіант</span>
              {/if}
            </div>
          </div>
          <div>
            <button
              class="btn-secondary"
              on:click={() => onSelectPreviewCandidate(candidate.documentId)}
              disabled={loading}
            >
              {selectedCandidateId === candidate.documentId ? "Вибрано" : "Обрати"}
            </button>
          </div>
        </div>
      {/each}
    </div>
  {:else if preview.decisionKind === "split"}
    <div class="documents-list">
      {#each preview.candidates as candidate}
        <div class="doc-row payment-row payment-row-matched">
          <div class="task-row-main">
            <div>
              <strong>{candidate.title}</strong>
              <p>{getPaymentDocumentKindLabel(candidate.documentKind)} • {candidate.openAmountStr}</p>
              <p>{getPaymentCandidateHint(candidate)}</p>
            </div>
            <div class="task-row-meta">
              <span class="task-pill">{PAYMENT_SCREEN_COPY.splitRecommendationBadge}</span>
            </div>
          </div>
        </div>
      {/each}
    </div>
    <div class="editor-actions">
      <button class="btn-secondary" on:click={onOpenManualPicker} disabled={loading}>
        {PAYMENT_SCREEN_COPY.chooseAnotherDocument}
      </button>
      <div class="empty-state-actions">
        <button class="btn-primary" type="button" on:click={onFocusImportButton}>Імпортувати виписку</button>
      </div>
    </div>
  {:else}
    <div class="editor-items-empty">
      <strong>{PAYMENT_SCREEN_COPY.emptyNoMatchTitle}</strong>
      <p>{PAYMENT_SCREEN_COPY.emptyNoMatchDescription}</p>
      <button class="btn-secondary" on:click={onOpenManualPicker} disabled={loading}>
        {PAYMENT_SCREEN_COPY.openManualSearch}
      </button>
    </div>
  {/if}

  {#if manualPicker}
    <PaymentManualMatchPanel
      {manualPicker}
      {loading}
      canConfirm={manualPickerCanConfirm}
      disabledReason={manualPickerDisabledReason}
      onSearchInput={onManualSearchInput}
      onRefreshSearch={onRefreshManualSearch}
      onAddToSplit={onAddManualCandidateToSplit}
      onConfirm={onConfirmManualCandidate}
      onClose={onCloseManualPicker}
      onSelectCandidate={onSelectManualCandidate}
    />
  {/if}

  {#if splitDraft}
    <PaymentSplitDraftPanel
      {splitDraft}
      {loading}
      onAllocationInput={onSplitAllocationInput}
      onRemoveAllocation={onRemoveSplitAllocation}
      onConfirm={onConfirmSplitDraft}
    />
  {/if}
</section>

<style>
  .chain-panel {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--acta-color-accent-soft) 32%, transparent), transparent 74%),
      var(--acta-color-bg-elevated);
  }

  .chain-panel-header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  .chain-panel-header p {
    margin: 6px 0 0;
    color: var(--acta-color-text-muted);
  }

  .documents-list {
    display: grid;
    gap: 16px;
  }

  .payment-row {
    display: flex;
    gap: 16px;
    justify-content: space-between;
    align-items: center;
    padding: 12px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 62%, var(--acta-color-bg-elevated));
  }

  .payment-row-matched {
    border-color: color-mix(in srgb, var(--acta-color-accent) 20%, var(--acta-color-border));
  }

  .editor-items-empty {
    display: grid;
    gap: 10px;
    padding: 20px;
    border-radius: var(--acta-radius-2xl);
    border: 1px dashed color-mix(in srgb, var(--acta-color-accent) 26%, var(--acta-color-border));
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 72%, var(--acta-color-bg-elevated));
  }

  .editor-actions,
  .task-row-meta {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }

  .task-row-meta,
  .editor-items-empty p {
    margin: 6px 0 0;
  }

  .payment-state {
    display: inline-flex;
    align-items: center;
    min-height: var(--acta-density-chip-h);
    padding: 0 10px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
  }

  .payment-state-matched {
    background: color-mix(in srgb, var(--acta-color-accent-soft) 70%, var(--acta-color-bg-elevated));
    color: var(--acta-color-accent-text);
  }

  @media (max-width: 1080px) {
    .chain-panel-header,
    .payment-row {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
