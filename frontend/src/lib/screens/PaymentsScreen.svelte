<script lang="ts">
  import PaymentCalendarPanel from "../components/PaymentCalendarPanel.svelte";
  import BankTabContent from "../components/BankTabContent.svelte";
  import { EDITOR_DIRTY_COPY } from "../config/ui";
  import { paymentsStore } from "../stores/payments";

  const payments = paymentsStore;
  let activeTab: "bank" | "calendar" = "bank";
  let pendingDirtyClose = false;

  $: busySave = $payments.loading && $payments.activeAction === "save";

  function closeEditor(force = false) {
    const result = payments.closeEditor(force);
    if (result && result.ok === false && result.reason === "dirty") {
      pendingDirtyClose = true;
      return result;
    }
    pendingDirtyClose = false;
    return result;
  }

  function requestCloseEditor() {
    closeEditor();
  }

  function confirmDiscardChanges() {
    closeEditor(true);
  }

  function cancelDiscardChanges() {
    pendingDirtyClose = false;
  }

  function onEditorBackdropClick() {
    requestCloseEditor();
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if ($payments.editor && event.key === "Escape") {
      requestCloseEditor();
    }
  }

  function onFieldInput(field: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLSelectElement;
    payments.updateFormField(field as never, input.value);
  }

  $: if (!$payments.editor && pendingDirtyClose) {
    pendingDirtyClose = false;
  }
</script>

<svelte:window on:keydown={onWindowKeydown} />

<section
  class="panel"
  data-testid="payments-screen"
  inert={$payments.editor ? true : undefined}
  aria-hidden={$payments.editor ? "true" : undefined}
>
  <div class="panel-header">
    <div>
      <h2>Платежі</h2>
      <p>{$payments.list?.items.length ?? 0} записів</p>
    </div>
  </div>

  <div class="payments-tabs" role="tablist">
    <button
      class="payments-tab"
      class:active={activeTab === "bank"}
      role="tab"
      aria-selected={activeTab === "bank"}
      on:click={() => (activeTab = "bank")}
    >Банк</button>
    <button
      class="payments-tab"
      class:active={activeTab === "calendar"}
      role="tab"
      aria-selected={activeTab === "calendar"}
      on:click={() => (activeTab = "calendar")}
    >Платіжний календар</button>
  </div>

  {#if activeTab === "bank"}
    <BankTabContent />
  {:else}
    <PaymentCalendarPanel />
  {/if}
</section>

{#if $payments.editor}
  <button
    type="button"
    class="editor-backdrop"
    aria-label="Закрити редактор"
    data-testid="payments-editor-backdrop"
    on:click={onEditorBackdropClick}
  ></button>
  <section class="editor-sheet" role="dialog" aria-modal="true">
    {#if pendingDirtyClose}
      <div
        class="editor-dirty-banner"
        role="alertdialog"
        aria-live="assertive"
        aria-labelledby="payments-dirty-banner-title"
        data-testid="payments-dirty-banner"
      >
        <div>
          <strong id="payments-dirty-banner-title">{EDITOR_DIRTY_COPY.dirtyTitle}</strong>
          <p>{EDITOR_DIRTY_COPY.dirtyDescription}</p>
        </div>
        <div class="editor-dirty-actions">
          <button
            type="button"
            class="btn-ghost btn-sm"
            on:click={cancelDiscardChanges}
            data-testid="payments-dirty-banner-cancel"
          >{EDITOR_DIRTY_COPY.dirtyStay}</button>
          <button
            type="button"
            class="btn-danger btn-sm"
            on:click={confirmDiscardChanges}
            data-testid="payments-dirty-banner-discard"
          >{EDITOR_DIRTY_COPY.dirtyDiscard}</button>
        </div>
      </div>
    {/if}
    <div class="editor-header">
      <div>
        <h3>{$payments.editor.id ? "Редагувати платіж" : "Новий платіж"}</h3>
        <p>Картка платежу</p>
      </div>
      <div class="editor-actions">
        <button class="btn-primary" on:click={() => payments.save()} disabled={busySave}>Зберегти</button>
        <button class="btn-ghost" on:click={requestCloseEditor} disabled={busySave}>Закрити</button>
      </div>
    </div>

    <div class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>Що перевірити перед збереженням</strong>
          <p>Перевірте напрям, суму, контрагента та референс, щоб звірка не губилася після імпорту.</p>
        </div>
        <div class="chain-summary">
          <div class="chain-summary-block">
            <span>Напрям</span>
            <strong>{$payments.editor.direction === "income" ? "Надходження" : "Витрата"}</strong>
          </div>
          <div class="chain-summary-block">
            <span>Пов'язаний документ</span>
            <strong>{$payments.editor.description || "Ще не вказано"}</strong>
          </div>
        </div>
      </div>
    </div>

    <div class="editor-grid payment-editor-grid">
      <label>
        Дата
        <input
          type="date"
          value={$payments.editor.date}
          on:input={(event) => onFieldInput("date", event)}
        />
      </label>
      <label>
        Напрям
        <select
          value={$payments.editor.direction}
          on:change={(event) => onFieldInput("direction", event)}
        >
          <option value="income">Надходження</option>
          <option value="expense">Витрата</option>
        </select>
      </label>
      <label>
        Сума
        <input
          value={$payments.editor.amount}
          on:input={(event) => onFieldInput("amount", event)}
        />
      </label>
      <label>
        Контрагент
        <select
          value={$payments.editor.counterpartyId}
          on:change={(event) => onFieldInput("counterpartyId", event)}
        >
          <option value="">- Без контрагента -</option>
          {#each $payments.list?.counterparties ?? [] as cp}
            <option value={cp.id}>{cp.name}</option>
          {/each}
        </select>
      </label>
      <label>
        Референс платежу
        <input
          value={$payments.editor.reference}
          on:input={(event) => onFieldInput("reference", event)}
        />
      </label>
      <label>
        Пов'язаний документ
        <input
          value={$payments.editor.description}
          on:input={(event) => onFieldInput("description", event)}
        />
      </label>
      <label class="editor-grid-span">
        Банк
        <input
          value={$payments.editor.bankName}
          on:input={(event) => onFieldInput("bankName", event)}
        />
      </label>
    </div>
  </section>
{/if}

<style>
  .payments-tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--acta-color-border);
    margin-top: 18px;
  }

  .payments-tab {
    padding: 10px 20px;
    border: 0;
    background: transparent;
    color: var(--acta-color-text-muted);
    font-weight: 500;
    cursor: pointer;
    position: relative;
  }

  .payments-tab:hover {
    color: var(--acta-color-text);
  }

  .payments-tab.active {
    color: var(--acta-color-accent-text);
  }

  .payments-tab.active::after {
    content: "";
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--acta-color-accent);
    border-radius: 2px 2px 0 0;
  }

  .editor-sheet {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-elevated);
    display: grid;
    gap: 16px;
  }

  .editor-header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  .editor-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }

  .editor-dirty-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    border-radius: var(--acta-radius-xl);
    background: color-mix(in srgb, var(--acta-color-danger-soft) 40%, var(--acta-color-bg-elevated));
    border: 1px solid color-mix(in srgb, var(--acta-color-danger) 22%, var(--acta-color-border));
  }

  .editor-dirty-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .chain-panel {
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

  .chain-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .chain-summary-block {
    display: grid;
    gap: 6px;
    padding: 14px 16px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 76%, var(--acta-color-bg-elevated));
  }

  .chain-summary-block span {
    font-size: 12px;
    color: var(--acta-color-text-muted);
  }

  .editor-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }

  .editor-grid label {
    display: grid;
    gap: 8px;
  }

  .editor-grid-span {
    grid-column: 1 / -1;
  }

  @media (max-width: 1080px) {
    .editor-grid,
    .chain-summary {
      grid-template-columns: 1fr;
    }

    .editor-header {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
