<script lang="ts">
  import SkeletonCard from "./SkeletonCard.svelte";
  import SkeletonRow from "./SkeletonRow.svelte";
  import {
    PAYMENT_SCREEN_COPY,
    PAYMENT_FLOW_COPY,
    PAYMENT_MANUAL_PICKER_DISABLED_REASON
  } from "../config/ui";
  import { isFormattedMoneyNegative } from "../money";
  import {
    getPaymentCandidateHint,
    getPaymentDirectionLabel,
    getPaymentDocumentKindLabel,
    getPaymentPreviewCopy,
    getPaymentStateLabel
  } from "../paymentsPresentation";
  import { paymentsStore } from "../stores/payments";
  import type { PaymentDraftFormDto } from "../types";

  const payments = paymentsStore;
  let importButton: HTMLButtonElement | null = null;

  function onPaymentFieldChange(field: keyof PaymentDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
    payments.updateFormField(field, input.value);
  }

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

  function isPaymentBusy(paymentId: string): boolean {
    return $payments.loading && $payments.activePaymentId === paymentId;
  }

  function openManualPickerForCurrentPreview() {
    const paymentId = $payments.matchPreview?.paymentId;
    if (paymentId) {
      payments.openManualMatchPicker(paymentId);
    }
  }

  function getFlowCopy(): { title: string; description: string } | null {
    if (!$payments.loading || !$payments.activeAction) {
      return null;
    }
    return PAYMENT_FLOW_COPY[$payments.activeAction] ?? null;
  }

  $: items = $payments.list?.items ?? [];
  $: unmatchedPayments = items.filter((item) => !item.matchedDoc);
  $: matchedPayments = items.filter((item) => Boolean(item.matchedDoc));
  $: busyImport = $payments.loading && $payments.activeAction === "import";
  $: busyImportPick = $payments.loading && $payments.activeAction === "import-pick";
  $: busyImportCommit = $payments.loading && $payments.activeAction === "import-commit";
  $: busySync = $payments.loading && $payments.activeAction === "sync";
  $: manualPickerCanConfirm = Boolean(
    $payments.manualPicker?.selectedCandidateId && ($payments.manualPicker?.candidates.length ?? 0) > 0
  );
  $: manualPickerDisabledReason =
    !$payments.manualPicker || $payments.manualPicker.candidates.length > 0
      ? ""
      : PAYMENT_MANUAL_PICKER_DISABLED_REASON;
  $: flowCopy = getFlowCopy();
  $: flowTitle = flowCopy?.title ?? null;
  $: flowDescription = flowCopy?.description ?? null;
  $: previewCopy = getPaymentPreviewCopy($payments.matchPreview);
</script>

<div class="bank-tab-root">
  <div class="payments-toolbar">
    <button
      bind:this={importButton}
      class="btn-primary"
      on:click={() => payments.pickAndPreviewImport()}
      disabled={busyImportPick || busyImport || busyImportCommit}
    >
      {busyImportPick ? PAYMENT_SCREEN_COPY.prepareImportPreview : PAYMENT_SCREEN_COPY.importStatement}
    </button>
    <button class="btn-secondary" on:click={() => payments.openEditor()} disabled={$payments.loading}>
      Створити платіж
    </button>
    <button
      class="btn-secondary"
      on:click={runHeaderReconciliation}
      disabled={unmatchedPayments.length === 0 || $payments.loading}
    >
      Запустити звірку{unmatchedPayments.length > 0 ? ` (${unmatchedPayments.length})` : ""}
    </button>
    <button
      class="btn-ghost"
      on:click={() => payments.importCsv()}
      disabled={busyImport || busyImportPick || busyImportCommit}
    >
      {busyImport ? PAYMENT_SCREEN_COPY.importing : PAYMENT_SCREEN_COPY.importFromStorage}
    </button>
    <button class="btn-ghost" on:click={() => payments.syncBank()} disabled={busyImport || busySync || busyImportPick}>
      {busySync ? PAYMENT_SCREEN_COPY.syncing : PAYMENT_SCREEN_COPY.syncWithBank}
    </button>
    <button class="btn-ghost" on:click={() => payments.openManualTemplate()} disabled={busyImport || busySync}>
      Шаблон CSV
    </button>
  </div>

  <div class="task-kpis" data-testid="payments-kpis">
    {#if $payments.initialLoading}
      <SkeletonCard count={4} />
    {:else}
      <div class="task-kpi-card">
        <strong>{$payments.list?.kpi.incomingStr ?? "0,00"}</strong>
        <span>{$payments.list?.kpi.incomingSub ?? "надходження"}</span>
      </div>
      <div class="task-kpi-card">
        <strong>{$payments.list?.kpi.outgoingStr ?? "0,00"}</strong>
        <span>{$payments.list?.kpi.outgoingSub ?? "витрати"}</span>
      </div>
      <div class="task-kpi-card">
        <strong>{$payments.list?.kpi.netStr ?? "0,00"}</strong>
        <span>Баланс</span>
      </div>
      <div class="task-kpi-card task-kpi-card-alert">
        <strong>{$payments.list?.kpi.unmatchedCount ?? 0}</strong>
        <span>Не зведено</span>
      </div>
    {/if}
  </div>

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
    <section class="chain-panel" data-testid="payments-import-preview" aria-label="Попередній перегляд імпорту виписки">
      <div class="chain-panel-header">
        <div>
          <strong>Попередній перегляд: {$payments.importPreview.bankName || "виписка"}</strong>
          <p>{$payments.importPreview.message}</p>
          <p class="payment-import-preview-path"><code>{$payments.importPreview.path}</code></p>
          {#if $payments.importPreviewStale}
            <div class="status-banner is-warning" role="status" aria-live="polite" data-testid="payments-import-preview-stale">
              <div>
                <strong>Файл виписки змінився</strong>
                <p>Перечитайте виписку, щоб оновити план імпорту перед підтвердженням.</p>
              </div>
            </div>
          {/if}
        </div>
        <div class="editor-actions">
          {#if $payments.importPreviewStale}
            <button
              class="btn-secondary"
              on:click={() => payments.refreshImportPreview()}
              disabled={busyImportPick || busyImportCommit}
            >
              Перечитати файл
            </button>
          {/if}
          <button
            class="btn-primary"
            on:click={() => payments.commitImportPreview()}
            disabled={busyImportCommit || $payments.importPreview.willCreate === 0 || $payments.importPreviewStale}
          >
            {busyImportCommit
              ? "Імпортуємо..."
              : $payments.importPreview.willCreate === 0
              ? "Немає нових платежів"
              : `Імпортувати ${$payments.importPreview.willCreate} платежі(ів)`}
          </button>
          <button
            class="btn-ghost"
            on:click={() => payments.cancelImportPreview()}
            disabled={busyImportCommit}
          >
            Скасувати
          </button>
        </div>
      </div>

      <ul class="payment-import-preview-summary">
        <li><span>Розпізнано рядків</span><strong>{$payments.importPreview.parsed}</strong></li>
        <li><span>Буде створено</span><strong>{$payments.importPreview.willCreate}</strong></li>
        <li><span>Уже існує (skip)</span><strong>{$payments.importPreview.willSkip}</strong></li>
        {#if $payments.importPreview.conflicts > 0}
          <li class="payment-import-preview-conflict"><span>Конфлікти</span><strong>{$payments.importPreview.conflicts}</strong></li>
        {/if}
      </ul>

      {#if $payments.importPreview.rows.length > 0}
        <table class="payment-import-preview-table">
          <thead>
            <tr>
              <th scope="col">Дія</th>
              <th scope="col">Bank ref</th>
              <th scope="col">Призначення</th>
              <th scope="col">Нотатка</th>
            </tr>
          </thead>
          <tbody>
            {#each $payments.importPreview.rows.slice(0, 25) as row, idx (idx)}
              <tr class:is-skipped={row.action === "skip"}>
                <td>{row.action === "create" ? "Нове" : "Пропуск"}</td>
                <td>{row.bankRef || "—"}</td>
                <td>{row.description || "—"}</td>
                <td>{row.note}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if $payments.importPreview.rows.length > 25}
          <p class="payment-import-preview-more">
            Показано перші 25 з {$payments.importPreview.rows.length} рядків.
          </p>
        {/if}
      {:else}
        <p class="payment-import-preview-empty">У файлі не знайдено жодного рядка виписки.</p>
      {/if}
    </section>
  {/if}

  {#if $payments.matchPreview}
    <section class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>{previewCopy?.title ?? ""}</strong>
          <p>{previewCopy?.description ?? ""}</p>
        </div>
        <div class="editor-actions">
          {#if $payments.matchPreview.decisionKind === "exact" && $payments.matchPreview.autoMatch}
            <button
              class="btn-primary"
              on:click={() => payments.confirmPreviewAutoMatch()}
              disabled={$payments.loading}
            >
              {PAYMENT_SCREEN_COPY.confirmAutoMatch}
            </button>
          {:else if $payments.matchPreview.decisionKind === "ambiguous"}
            <button
              class="btn-primary"
              on:click={() => payments.confirmSelectedPreviewCandidate()}
              disabled={$payments.loading}
            >
              {PAYMENT_SCREEN_COPY.confirmPreviewCandidate}
            </button>
            <button class="btn-secondary" on:click={openManualPickerForCurrentPreview} disabled={$payments.loading}>
              {PAYMENT_SCREEN_COPY.chooseAnotherDocument}
            </button>
          {/if}
          <button class="btn-ghost" on:click={() => payments.closeMatchPreview()} disabled={$payments.loading}>
            {PAYMENT_SCREEN_COPY.closePreview}
          </button>
        </div>
      </div>

      {#if $payments.matchPreview.decisionKind === "exact" && $payments.matchPreview.autoMatch}
        <div class="doc-row payment-row payment-row-matched">
          <div class="task-row-main">
            <div>
              <strong>{$payments.matchPreview.autoMatch.title}</strong>
              <p>{getPaymentDocumentKindLabel($payments.matchPreview.autoMatch.documentKind)} • {$payments.matchPreview.autoMatch.amountStr}</p>
            </div>
            <div class="task-row-meta">
              <span class="task-pill">Рекомендація</span>
              <span>Автопідтвердження доступне</span>
            </div>
          </div>
        </div>
      {:else if $payments.matchPreview.decisionKind === "ambiguous"}
        <div class="documents-list">
          {#each $payments.matchPreview.candidates as candidate}
            <div
              class="doc-row payment-row"
              class:payment-row-matched={$payments.selectedCandidateId === candidate.documentId}
            >
              <div class="task-row-main">
                <div>
                  <strong>{candidate.title}</strong>
                  <p>{getPaymentDocumentKindLabel(candidate.documentKind)} • {candidate.openAmountStr}</p>
                  <p>{getPaymentCandidateHint(candidate)}</p>
                </div>
                <div class="task-row-meta">
                  <span class="task-pill">Скоринг {candidate.totalScore.toFixed(2)}</span>
                  {#if $payments.selectedCandidateId === candidate.documentId}
                    <span class="payment-state payment-state-matched">Обраний варіант</span>
                  {/if}
                </div>
              </div>
              <div>
                <button
                  class="btn-secondary"
                  on:click={() => payments.selectPreviewCandidate(candidate.documentId)}
                  disabled={$payments.loading}
                >
                  {$payments.selectedCandidateId === candidate.documentId ? "Вибрано" : "Обрати"}
                </button>
              </div>
            </div>
          {/each}
        </div>
      {:else if $payments.matchPreview.decisionKind === "split"}
        <div class="documents-list">
          {#each $payments.matchPreview.candidates as candidate}
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
          <button class="btn-secondary" on:click={openManualPickerForCurrentPreview} disabled={$payments.loading}>
            {PAYMENT_SCREEN_COPY.chooseAnotherDocument}
          </button>
          <div class="empty-state-actions">
            <button class="btn-primary" type="button" on:click={focusImportButton}>Імпортувати виписку</button>
          </div>
        </div>
      {:else}
        <div class="editor-items-empty">
          <strong>{PAYMENT_SCREEN_COPY.emptyNoMatchTitle}</strong>
          <p>{PAYMENT_SCREEN_COPY.emptyNoMatchDescription}</p>
          <button class="btn-secondary" on:click={openManualPickerForCurrentPreview} disabled={$payments.loading}>
            {PAYMENT_SCREEN_COPY.openManualSearch}
          </button>
        </div>
      {/if}

      {#if $payments.manualPicker}
        <section class="editor-items-empty" data-testid="payments-manual-picker">
          <strong>{PAYMENT_SCREEN_COPY.manualPickerTitle}</strong>
          <p>{PAYMENT_SCREEN_COPY.manualPickerDescription}</p>
          <div class="editor-grid">
            <label class="editor-grid-span">
              Пошук
              <input
                value={$payments.manualPicker.query}
                on:input={onManualSearchInput}
                placeholder="ACT-001, INV-002, оплата..."
              />
            </label>
          </div>
          <div class="editor-actions">
            <button class="btn-secondary" on:click={() => payments.searchManualMatchCandidates()} disabled={$payments.loading}>
              {PAYMENT_SCREEN_COPY.refreshManualSearch}
            </button>
              <button class="btn-secondary" on:click={() => payments.addSelectedManualPickerCandidateToSplit()} disabled={$payments.loading}>
                Додати до розподілу
              </button>
              <button
                class="btn-primary"
                on:click={() => payments.confirmManualPickerCandidate()}
                aria-describedby={!manualPickerCanConfirm && manualPickerDisabledReason ? "payments-manual-picker-hint" : undefined}
                disabled={$payments.loading || !manualPickerCanConfirm}
            >
              {PAYMENT_SCREEN_COPY.confirmManualDocument}
            </button>
            <button class="btn-ghost" on:click={() => payments.closeManualMatchPicker()} disabled={$payments.loading}>
              {PAYMENT_SCREEN_COPY.closeManualSearch}
            </button>
          </div>

          {#if !manualPickerCanConfirm && manualPickerDisabledReason}
            <p id="payments-manual-picker-hint">{manualPickerDisabledReason}</p>
          {/if}

          {#if $payments.manualPicker.candidates.length === 0}
            <p>{PAYMENT_SCREEN_COPY.emptyManualSearch}</p>
          {:else}
            <div class="documents-list">
              {#each $payments.manualPicker.candidates as candidate}
                <div
                  class="doc-row payment-row"
                  class:payment-row-matched={$payments.manualPicker.selectedCandidateId === candidate.documentId}
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
                      on:click={() => payments.selectManualPickerCandidate(candidate.documentId)}
                      disabled={$payments.loading}
                    >
                      {$payments.manualPicker.selectedCandidateId === candidate.documentId ? "Вибрано" : "Обрати"}
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/if}

      {#if $payments.splitDraft}
        <section class="editor-items-empty" data-testid="payments-split-draft">
          <strong>{PAYMENT_SCREEN_COPY.splitDraftTitle}</strong>
          <p>
            Сума платежу: {$payments.splitDraft.paymentAmountStr}
            • Залишок: {$payments.splitDraft.remainingAmountStr}
          </p>

          {#if $payments.splitDraft.allocations.length === 0}
            <p>Додайте документи з manual picker, щоб сформувати розподіл.</p>
          {:else}
            <div class="documents-list">
              {#each $payments.splitDraft.allocations as allocation}
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
                          on:input={(event) => onSplitAllocationInput(allocation.documentId, event)}
                        />
                      </label>
                    </div>
                  </div>
                  <div>
                    <button class="btn-ghost" on:click={() => payments.removeSplitAllocation(allocation.documentId)} disabled={$payments.loading}>
                      Прибрати
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          <div class="editor-actions">
            <button class="btn-primary" on:click={() => payments.confirmSplitDraft()} disabled={$payments.loading || $payments.splitDraft.allocations.length === 0}>
              {PAYMENT_SCREEN_COPY.confirmSplit}
            </button>
          </div>
        </section>
      {/if}
    </section>
  {/if}

  <div class="payments-groups">
    <section class="payments-group payments-group-unmatched" data-testid="payments-unmatched-group">
      <div class="payments-group-header">
        <div>
          <strong>Потребують звірки</strong>
          <p>Починайте саме з цих рухів: вони ще не пов'язані з документами.</p>
        </div>
        <span class="payment-group-count">{unmatchedPayments.length}</span>
      </div>

      {#if $payments.initialLoading}
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
              <button class="task-row-main payment-row-main" on:click={() => payments.openEditor(item)}>
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
                  on:click={() => payments.reconcile(item.id)}
                  disabled={isPaymentBusy(item.id)}
                >
                  {isPaymentBusy(item.id) ? PAYMENT_SCREEN_COPY.reconcileAction : PAYMENT_SCREEN_COPY.reconcileIdle}
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

      {#if $payments.initialLoading}
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
              <button class="task-row-main payment-row-main" on:click={() => payments.openEditor(item)}>
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
                  on:click={() => payments.unreconcile(item.id)}
                  disabled={isPaymentBusy(item.id)}
                >
                  {isPaymentBusy(item.id) ? "Знімаємо..." : "Зняти зведення"}
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .payments-toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 18px;
  }

  .payments-group,
  .flow-banner,
  .chain-panel {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-elevated);
  }

  .flow-banner,
  .chain-panel {
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--acta-color-accent-soft) 32%, transparent), transparent 74%),
      var(--acta-color-bg-elevated);
  }

  .payments-group-header,
  .chain-panel-header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  .payments-group-header p,
  .chain-panel-header p,
  .flow-banner p {
    margin: 6px 0 0;
    color: var(--acta-color-text-muted);
  }

  .payments-groups,
  .task-kpis,
  .documents-list {
    display: grid;
    gap: 16px;
  }

  .task-kpis {
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

  .task-kpi-card-alert,
  .payments-group-unmatched {
    border-color: color-mix(in srgb, var(--acta-color-accent) 22%, var(--acta-color-border));
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--acta-color-accent-soft) 36%, transparent), transparent 80%),
      var(--acta-color-bg-elevated);
  }

  .payment-group-count,
  .task-kpi-card span {
    font-size: 12px;
    color: var(--acta-color-text-muted);
  }

  .payment-row-actions,
  .editor-actions,
  .task-row-meta {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
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
    font-weight: 700;
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
    .task-kpis {
      grid-template-columns: 1fr;
    }

    .payments-group-header,
    .chain-panel-header,
    .payment-row {
      flex-direction: column;
      align-items: flex-start;
    }
  }

  .payment-import-preview-path {
    font-size: 0.85em;
    color: var(--acta-color-text-muted);
    margin: 0.25rem 0 0 0;
    overflow-wrap: anywhere;
  }

  .payment-import-preview-summary {
    display: flex;
    flex-wrap: wrap;
    gap: 1.5rem;
    list-style: none;
    padding: 0;
    margin: 1rem 0 0.75rem 0;
  }

  .payment-import-preview-summary li {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .payment-import-preview-summary li span {
    font-size: 0.78em;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--acta-color-text-muted);
  }

  .payment-import-preview-summary li strong {
    font-size: 1.25rem;
  }

  .payment-import-preview-summary li.payment-import-preview-conflict strong {
    color: var(--acta-color-danger);
  }

  .payment-import-preview-table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.75rem;
    font-size: 0.92rem;
  }

  .payment-import-preview-table thead {
    background: var(--acta-color-bg-subtle);
  }

  .payment-import-preview-table th,
  .payment-import-preview-table td {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--acta-color-border);
    vertical-align: top;
  }

  .payment-import-preview-table tr.is-skipped {
    color: var(--acta-color-text-muted);
  }

  .payment-import-preview-more,
  .payment-import-preview-empty {
    margin: 0.5rem 0 0 0;
    font-size: 0.88rem;
    color: var(--acta-color-text-muted);
  }
</style>
