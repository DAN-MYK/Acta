<script lang="ts">
  import { PAYMENT_SCREEN_COPY } from "../../config/ui";
  import {
    getPaymentCandidateHint,
    getPaymentDocumentKindLabel
  } from "../../paymentsPresentation";
  import type { PaymentManualPickerViewState } from "./payment-view-model";

  export let manualPicker: PaymentManualPickerViewState;
  export let loading = false;
  export let canConfirm = false;
  export let disabledReason = "";
  export let onSearchInput: (event: Event) => void;
  export let onRefreshSearch: () => void;
  export let onAddToSplit: () => void;
  export let onConfirm: () => void;
  export let onClose: () => void;
  export let onSelectCandidate: (documentId: string) => void;
</script>

<section class="editor-items-empty" data-testid="payments-manual-picker">
  <strong>{PAYMENT_SCREEN_COPY.manualPickerTitle}</strong>
  <p>{PAYMENT_SCREEN_COPY.manualPickerDescription}</p>
  <div class="editor-grid">
    <label class="editor-grid-span">
      Пошук
      <input
        value={manualPicker.query}
        on:input={onSearchInput}
        placeholder="ACT-001, INV-002, оплата..."
      />
    </label>
  </div>
  <div class="editor-actions">
    <button class="btn-secondary" on:click={onRefreshSearch} disabled={loading}>
      {PAYMENT_SCREEN_COPY.refreshManualSearch}
    </button>
    <button class="btn-secondary" on:click={onAddToSplit} disabled={loading}>
      Додати до розподілу
    </button>
    <button
      class="btn-primary"
      on:click={onConfirm}
      aria-describedby={!canConfirm && disabledReason ? "payments-manual-picker-hint" : undefined}
      disabled={loading || !canConfirm}
    >
      {PAYMENT_SCREEN_COPY.confirmManualDocument}
    </button>
    <button class="btn-ghost" on:click={onClose} disabled={loading}>
      {PAYMENT_SCREEN_COPY.closeManualSearch}
    </button>
  </div>

  {#if !canConfirm && disabledReason}
    <p id="payments-manual-picker-hint">{disabledReason}</p>
  {/if}

  {#if manualPicker.candidates.length === 0}
    <p>{PAYMENT_SCREEN_COPY.emptyManualSearch}</p>
  {:else}
    <div class="documents-list">
      {#each manualPicker.candidates as candidate}
        <div
          class="doc-row payment-row"
          class:payment-row-matched={manualPicker.selectedCandidateId === candidate.documentId}
        >
          <div class="task-row-main">
            <div>
              <strong>{candidate.title}</strong>
              <p>{getPaymentDocumentKindLabel(candidate.documentKind)} • {candidate.openAmountStr}</p>
              <p>{getPaymentCandidateHint(candidate)}</p>
            </div>
            <div class="task-row-meta">
              <span class="task-pill">Скоринг {candidate.totalScore.toFixed(2)}</span>
            </div>
          </div>
          <div>
            <button
              class="btn-secondary"
              on:click={() => onSelectCandidate(candidate.documentId)}
              disabled={loading}
            >
              {manualPicker.selectedCandidateId === candidate.documentId ? "Вибрано" : "Обрати"}
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .editor-items-empty {
    display: grid;
    gap: 10px;
    padding: 20px;
    border-radius: var(--acta-radius-2xl);
    border: 1px dashed color-mix(in srgb, var(--acta-color-accent) 26%, var(--acta-color-border));
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 72%, var(--acta-color-bg-elevated));
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

  @media (max-width: 1080px) {
    .payment-row {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
