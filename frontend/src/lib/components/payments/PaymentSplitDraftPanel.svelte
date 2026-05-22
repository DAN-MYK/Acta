<script lang="ts">
  import { PAYMENT_SCREEN_COPY } from "../../config/ui";
  import { getPaymentDocumentKindLabel } from "../../paymentsPresentation";
  import type { PaymentSplitDraftViewState } from "./payment-view-model";

  export let splitDraft: PaymentSplitDraftViewState;
  export let loading = false;
  export let onAllocationInput: (documentId: string, event: Event) => void;
  export let onRemoveAllocation: (documentId: string) => void;
  export let onConfirm: () => void;
</script>

<section class="editor-items-empty" data-testid="payments-split-draft">
  <strong>{PAYMENT_SCREEN_COPY.splitDraftTitle}</strong>
  <p>
    Сума платежу: {splitDraft.paymentAmountStr}
    • Залишок: {splitDraft.remainingAmountStr}
  </p>

  {#if splitDraft.allocations.length === 0}
    <p>Додайте документи з manual picker, щоб сформувати розподіл.</p>
  {:else}
    <div class="documents-list">
      {#each splitDraft.allocations as allocation}
        <div class="doc-row payment-row">
          <div class="task-row-main">
            <div>
              <strong>{allocation.title}</strong>
              <p>{getPaymentDocumentKindLabel(allocation.documentKind)} • Залишок документа: {allocation.openAmountStr}</p>
            </div>
            <div class="task-row-meta">
              <label>
                <span>Сума</span>
                <input
                  value={allocation.amount}
                  on:input={(event) => onAllocationInput(allocation.documentId, event)}
                />
              </label>
            </div>
          </div>
          <div>
            <button class="btn-ghost" on:click={() => onRemoveAllocation(allocation.documentId)} disabled={loading}>
              Прибрати
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="editor-actions">
    <button class="btn-primary" on:click={onConfirm} disabled={loading || splitDraft.allocations.length === 0}>
      {PAYMENT_SCREEN_COPY.confirmSplit}
    </button>
  </div>
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
