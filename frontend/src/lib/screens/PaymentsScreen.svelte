<script lang="ts">
  import { paymentsStore } from "../stores/payments";
  import type { PaymentDraftFormDto, PaymentMatchCandidateDto } from "../types";

  const payments = paymentsStore;

  function onPaymentFieldChange(field: keyof PaymentDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
    payments.updateFormField(field, input.value);
  }

  function getPaymentStateLabel(matchedDoc: string): string {
    return matchedDoc ? `Зв'язано з ${matchedDoc}` : "Не зведено";
  }

  function getDocumentKindLabel(kind: PaymentMatchCandidateDto["documentKind"]): string {
    return kind === "act" ? "Акт" : "Накладна";
  }

  function getPreviewTitle(): string {
    const preview = $payments.matchPreview;
    if (!preview) {
      return "";
    }

    if (preview.decisionKind === "exact") {
      return "Рекомендована звірка";
    }

    if (preview.decisionKind === "ambiguous") {
      return "Кілька кандидатів на звірку — потрібна увага";
    }

    return "Точний кандидат не знайдений";
  }

  function getPreviewDescription(): string {
    const preview = $payments.matchPreview;
    if (!preview) {
      return "";
    }

    if (preview.decisionKind === "exact") {
      return "Система знайшла найкращий документ для автозіставлення. Перевірте рекомендацію перед підтвердженням.";
    }

    if (preview.decisionKind === "ambiguous") {
      return "Оберіть найкращий варіант у списку. Цей платіж потребує уваги, а ручне підтвердження звірки буде наступним кроком.";
    }

    return "Для цього платежу поки немає точного збігу. Перевірте реквізити або підготуйте ручне звіряння.";
  }

  function getCandidateHint(candidate: PaymentMatchCandidateDto): string {
    const hints: string[] = [];

    if (candidate.sameIban) {
      hints.push("та самий IBAN");
    }

    if (candidate.referenceHit) {
      hints.push("є збіг по призначенню");
    }

    if (candidate.textHits > 0) {
      hints.push(`текстових збігів: ${candidate.textHits}`);
    }

    hints.push(`відхилення по даті: ${candidate.daysDistance} дн.`);
    return hints.join(" • ");
  }
</script>

<section class="panel" data-testid="payments-screen">
  <div class="panel-header">
    <div>
      <h2>Платежі</h2>
      <p>{$payments.list?.items.length ?? 0} записів</p>
    </div>
    <div class="panel-actions">
      <button class="btn-secondary" on:click={() => payments.importCsv()}>Імпортувати виписку</button>
      <button class="btn-ghost" on:click={() => payments.syncBank()}>Оновити з банку</button>
      <button class="btn-ghost" on:click={() => payments.openManualTemplate()}>Шаблон CSV</button>
      <button class="btn-primary" on:click={() => payments.openEditor()}>Створити платіж</button>
    </div>
  </div>

  <div class="create-strip-card">
    <div class="create-strip-header">
      <div>
        <strong>Контроль руху грошей</strong>
        <p>1. Імпорт  2. Звірка  3. Ручний платіж</p>
      </div>
      <span class="doc-kind-badge">Звірка в центрі уваги</span>
    </div>

    <p class="create-strip-hint">
      Імпортуйте виписку, швидко знайдіть незведені рухи та лише потім додавайте ручні коригування.
    </p>
  </div>

  <div class="task-kpis">
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
    <div class="task-kpi-card">
      <strong>{$payments.list?.kpi.unmatchedCount ?? 0}</strong>
      <span>Не зведено</span>
    </div>
  </div>

  {#if $payments.message}
    <p class="message">{$payments.message}</p>
  {/if}

  {#if $payments.error}
    <p class="error">{$payments.error}</p>
  {/if}

  {#if $payments.loading}
    <p class="message">Оновлюємо платежі та статуси звірки…</p>
  {/if}

  {#if $payments.matchPreview}
    <section class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>{getPreviewTitle()}</strong>
          <p>{getPreviewDescription()}</p>
        </div>
        <div class="editor-actions">
          {#if $payments.matchPreview.decisionKind === "exact" && $payments.matchPreview.autoMatch}
            <button class="btn-primary" on:click={() => payments.confirmPreviewAutoMatch()}>
              Підтвердити автозіставлення
            </button>
          {/if}
          <button class="btn-ghost" on:click={() => payments.closeMatchPreview()}>Закрити preview</button>
        </div>
      </div>

      {#if $payments.matchPreview.decisionKind === "exact" && $payments.matchPreview.autoMatch}
        <div class="doc-row payment-row payment-row-matched">
          <div class="task-row-main">
            <div>
              <strong>{$payments.matchPreview.autoMatch.title}</strong>
              <p>{getDocumentKindLabel($payments.matchPreview.autoMatch.documentKind)} • {$payments.matchPreview.autoMatch.amountStr}</p>
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
                  <p>{getDocumentKindLabel(candidate.documentKind)} • {candidate.openAmountStr}</p>
                  <p>{getCandidateHint(candidate)}</p>
                </div>
                <div class="task-row-meta">
                  <span class="task-pill">Скоринг {candidate.totalScore.toFixed(2)}</span>
                  {#if $payments.selectedCandidateId === candidate.documentId}
                    <span class="payment-state payment-state-matched">Обраний варіант</span>
                  {/if}
                </div>
              </div>
              <div>
                <button class="btn-secondary" on:click={() => payments.selectPreviewCandidate(candidate.documentId)}>
                  {$payments.selectedCandidateId === candidate.documentId ? "Вибрано" : "Обрати"}
                </button>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="editor-items-empty">
          <strong>Автоматична звірка не знайшла точного документа</strong>
          <p>Поки що backend не викликається для ручного підтвердження з цього екрана. Можна переглянути платіж і підготувати наступний крок вручну.</p>
        </div>
      {/if}
    </section>
  {/if}

  <div class="documents-list">
    {#if ($payments.list?.items.length ?? 0) === 0}
      <div class="editor-items-empty">
        <strong>Ще немає жодного платежу</strong>
        <p>Імпортуйте виписку або створіть ручний платіж, щоб почати звірку руху грошей.</p>
      </div>
    {:else}
      {#each $payments.list?.items ?? [] as item}
        <div
          class="doc-row payment-row"
          class:payment-row-matched={Boolean(item.matchedDoc)}
          class:payment-row-unmatched={!item.matchedDoc}
        >
          <button class="task-row-main" on:click={() => payments.openEditor(item)}>
            <div>
              <strong>{item.date} - {item.counterparty || "Без контрагента"}</strong>
              <p>{item.account || ""}</p>
            </div>
            <div class="task-row-meta">
              <span class="task-pill">{item.direction === "in" ? "Надходження" : "Витрата"}</span>
              <span>{item.amountStr}</span>
              <span
                class="payment-state"
                class:payment-state-matched={Boolean(item.matchedDoc)}
                class:payment-state-unmatched={!item.matchedDoc}
              >
                {getPaymentStateLabel(item.matchedDoc)}
              </span>
            </div>
          </button>
          <div>
            {#if item.matchedDoc}
              <button class="btn-ghost" on:click={() => payments.unreconcile(item.id)}>Зняти звірку</button>
            {:else}
              <button class="btn-secondary" on:click={() => payments.reconcile(item.id)}>
                {$payments.matchPreview?.paymentId === item.id ? "Оновити preview" : "Звірити платіж"}
              </button>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</section>

{#if $payments.editor}
  <section class="editor-sheet">
    <div class="editor-header">
      <div>
        <h3>{$payments.editor.id ? "Редагувати платіж" : "Новий платіж"}</h3>
        <p>Картка платежу</p>
      </div>
      <div class="editor-actions">
        <button class="btn-primary" on:click={() => payments.save()}>Зберегти</button>
        <button class="btn-ghost" on:click={() => payments.closeEditor()}>Закрити</button>
      </div>
    </div>

    <div class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>Що перевірити перед збереженням</strong>
          <p>Перевірте напрям, суму, контрагента та джерело платежу, щоб звірка не губилася після імпорту.</p>
        </div>
        <div class="chain-summary">
          <div class="chain-summary-block">
            <span>Напрям</span>
            <strong>{$payments.editor.direction === "income" ? "Надходження" : "Витрата"}</strong>
          </div>
          <div class="chain-summary-block">
            <span>Пов'язаний документ</span>
            <strong>{$payments.editor.reference || "Ще не вказано"}</strong>
          </div>
        </div>
      </div>
    </div>

    <div class="editor-grid">
      <label>
        Дата
        <input type="date" value={$payments.editor.date} on:input={(event) => onPaymentFieldChange("date", event)} />
      </label>
      <label>
        Сума
        <input value={$payments.editor.amount} on:input={(event) => onPaymentFieldChange("amount", event)} />
      </label>
      <label>
        Напрям
        <select value={$payments.editor.direction} on:change={(event) => onPaymentFieldChange("direction", event)}>
          <option value="income">Надходження</option>
          <option value="expense">Витрата</option>
        </select>
      </label>
      <label>
        Контрагент
        <select value={$payments.editor.counterpartyId} on:change={(event) => onPaymentFieldChange("counterpartyId", event)}>
          <option value="">- Без контрагента -</option>
          {#each $payments.list?.counterparties ?? [] as cp}
            <option value={cp.id}>{cp.name}</option>
          {/each}
        </select>
      </label>
      <label>
        Банк
        <input value={$payments.editor.bankName} on:input={(event) => onPaymentFieldChange("bankName", event)} />
      </label>
      <label>
        Пов'язаний документ
        <input value={$payments.editor.reference} on:input={(event) => onPaymentFieldChange("reference", event)} />
      </label>
      <label class="editor-grid-span">
        Опис
        <textarea rows="3" value={$payments.editor.description} on:input={(event) => onPaymentFieldChange("description", event)}></textarea>
      </label>
    </div>
  </section>
{/if}
