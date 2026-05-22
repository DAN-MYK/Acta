<script lang="ts">
  import { PAYMENT_SCREEN_COPY } from "../../config/ui";
  import { isFormattedMoneyNegative } from "../../money";
  import {
    getPaymentDirectionLabel,
    getPaymentStateLabel
  } from "../../paymentsPresentation";
  import type { PaymentItemDto } from "../../types";
  import SkeletonRow from "../SkeletonRow.svelte";
  import { isPaymentBusy } from "./payment-view-model";

  export let unmatchedPayments: PaymentItemDto[] = [];
  export let matchedPayments: PaymentItemDto[] = [];
  export let initialLoading = false;
  export let loading = false;
  export let activePaymentId: string | null = null;
  export let onOpenEditor: (payment: PaymentItemDto) => void;
  export let onReconcile: (paymentId: string) => void;
  export let onUnreconcile: (paymentId: string) => void;

  function paymentBusy(paymentId: string): boolean {
    return isPaymentBusy(loading, activePaymentId, paymentId);
  }
</script>

<div class="payments-groups">
  <section class="payments-group payments-group-unmatched" data-testid="payments-unmatched-group">
    <div class="payments-group-header">
      <div>
        <strong>Потребують звірки</strong>
        <p>Починайте саме з цих рухів: вони ще не пов'язані з документами.</p>
      </div>
      <span class="payment-group-count">{unmatchedPayments.length}</span>
    </div>

    {#if initialLoading}
      <SkeletonRow count={3} />
    {:else if unmatchedPayments.length === 0}
      <div class="editor-items-empty">
        <span class="empty-state-eyebrow">Додайте перший рух</span>
        <strong>{PAYMENT_SCREEN_COPY.emptyUnmatchedTitle}</strong>
        <p>{PAYMENT_SCREEN_COPY.emptyUnmatchedDescription}</p>
      </div>
    {:else}
      <div class="documents-list">
        {#each unmatchedPayments as item (item.id)}
          <div class="doc-row payment-row payment-row-unmatched">
            <button class="task-row-main payment-row-main" on:click={() => onOpenEditor(item)}>
              <div>
                <strong>{item.date} - {item.counterparty || "Без контрагента"}</strong>
                <p>{item.account || "Банк не вказано"}</p>
              </div>
              <div class="task-row-meta">
                <span class="task-pill">{getPaymentDirectionLabel(item.direction)}</span>
                <span class="money-value" data-negative={isFormattedMoneyNegative(item.amountStr)}>{item.amountStr}</span>
                <span class="payment-state payment-state-unmatched">{getPaymentStateLabel(item.matchedDoc)}</span>
              </div>
            </button>
            <div class="payment-row-actions">
              <button
                class="btn-primary"
                on:click={() => onReconcile(item.id)}
                disabled={paymentBusy(item.id)}
              >
                {paymentBusy(item.id) ? PAYMENT_SCREEN_COPY.reconcileAction : PAYMENT_SCREEN_COPY.reconcileIdle}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <section class="payments-group" data-testid="payments-matched-group">
    <div class="payments-group-header">
      <div>
        <strong>Вже зведені</strong>
        <p>Тут лишаються платежі, які вже мають зв'язок із документом і потребують лише контролю.</p>
      </div>
      <span class="payment-group-count">{matchedPayments.length}</span>
    </div>

    {#if initialLoading}
      <SkeletonRow count={3} />
    {:else if matchedPayments.length === 0}
      <div class="editor-items-empty">
        <strong>{PAYMENT_SCREEN_COPY.emptyMatchedTitle}</strong>
        <p>{PAYMENT_SCREEN_COPY.emptyMatchedDescription}</p>
      </div>
    {:else}
      <div class="documents-list">
        {#each matchedPayments as item (item.id)}
          <div class="doc-row payment-row payment-row-matched">
            <button class="task-row-main payment-row-main" on:click={() => onOpenEditor(item)}>
              <div>
                <strong>{item.date} - {item.counterparty || "Без контрагента"}</strong>
                <p>{item.account || "Банк не вказано"}</p>
              </div>
              <div class="task-row-meta">
                <span class="task-pill">{getPaymentDirectionLabel(item.direction)}</span>
                <span class="money-value" data-negative={isFormattedMoneyNegative(item.amountStr)}>{item.amountStr}</span>
                <span class="payment-state payment-state-matched">{getPaymentStateLabel(item.matchedDoc)}</span>
              </div>
            </button>
            <div class="payment-row-actions">
              <button
                class="btn-secondary"
                on:click={() => onUnreconcile(item.id)}
                disabled={paymentBusy(item.id)}
              >
                {paymentBusy(item.id) ? "Знімаємо..." : "Зняти зведення"}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .payments-group {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-elevated);
  }

  .payments-group-header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  .payments-group-header p {
    margin: 6px 0 0;
    color: var(--acta-color-text-muted);
  }

  .payments-groups,
  .documents-list {
    display: grid;
    gap: 16px;
  }

  .payments-group-unmatched {
    border-color: color-mix(in srgb, var(--acta-color-accent) 22%, var(--acta-color-border));
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--acta-color-accent-soft) 36%, transparent), transparent 80%),
      var(--acta-color-bg-elevated);
  }

  .payment-group-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 40px;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 82%, var(--acta-color-bg-elevated));
    color: var(--acta-color-text-muted);
    font-size: 12px;
    font-weight: 700;
  }

  .payment-row-actions,
  .task-row-meta {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
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

  .payment-row-unmatched {
    border-color: color-mix(in srgb, var(--acta-color-danger) 24%, var(--acta-color-border));
  }

  .payment-row-matched {
    border-color: color-mix(in srgb, var(--acta-color-accent) 20%, var(--acta-color-border));
  }

  .payment-row-main {
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .payment-row-main p,
  .task-row-meta,
  .editor-items-empty p {
    margin: 6px 0 0;
  }

  .payment-state-unmatched {
    background: color-mix(in srgb, var(--acta-color-danger) 12%, var(--acta-color-bg-elevated));
    color: color-mix(in srgb, var(--acta-color-danger) 72%, var(--acta-color-text));
  }

  .payment-state-matched {
    background: color-mix(in srgb, var(--acta-color-accent-soft) 70%, var(--acta-color-bg-elevated));
    color: var(--acta-color-accent-text);
  }

  .editor-items-empty {
    display: grid;
    gap: 10px;
    padding: 20px;
    border-radius: var(--acta-radius-2xl);
    border: 1px dashed color-mix(in srgb, var(--acta-color-accent) 26%, var(--acta-color-border));
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 72%, var(--acta-color-bg-elevated));
  }

  @media (max-width: 1080px) {
    .payments-group-header,
    .payment-row {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
