<script lang="ts">
  import { paymentsStore } from "../stores/payments";
  import type { PaymentDraftFormDto } from "../types";

  const payments = paymentsStore;

  function onPaymentFieldChange(field: keyof PaymentDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
    payments.updateFormField(field, input.value);
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Платежі</h2>
      <p>{$payments.list?.items.length ?? 0} записів</p>
    </div>
    <div class="panel-actions">
      <button on:click={() => payments.importCsv()}>Імпорт CSV</button>
      <button on:click={() => payments.syncBank()}>Синхронізувати банк</button>
      <button on:click={() => payments.openManualTemplate()}>Шаблон CSV</button>
      <button on:click={() => payments.openEditor()}>Новий платіж</button>
    </div>
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
      <span>Net</span>
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

  <div class="documents-list">
    {#each $payments.list?.items ?? [] as item}
      <div class="doc-row">
        <button class="task-row-main" on:click={() => payments.openEditor(item)}>
          <div>
            <strong>{item.date} - {item.counterparty || "Без контрагента"}</strong>
            <p>{item.account || ""}</p>
          </div>
          <div class="task-row-meta">
            <span class="task-pill">{item.direction === "in" ? "Надходження" : "Витрата"}</span>
            <span>{item.amountStr}</span>
            {#if item.matchedDoc}
              <span>Зведено</span>
            {/if}
          </div>
        </button>
        <div>
          {#if item.matchedDoc}
            <button on:click={() => payments.unreconcile(item.id)}>Зняти зведення</button>
          {:else}
            <button on:click={() => payments.reconcile(item.id)}>Звести</button>
          {/if}
        </div>
      </div>
    {/each}
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
        <button on:click={() => payments.save()}>Зберегти</button>
        <button on:click={() => payments.closeEditor()}>Закрити</button>
      </div>
    </div>

    <div class="editor-grid">
      <label>
        Дата
        <input value={$payments.editor.date} on:input={(event) => onPaymentFieldChange("date", event)} />
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
        Референс
        <input value={$payments.editor.reference} on:input={(event) => onPaymentFieldChange("reference", event)} />
      </label>
      <label class="editor-grid-span">
        Опис
        <textarea rows="3" value={$payments.editor.description} on:input={(event) => onPaymentFieldChange("description", event)}></textarea>
      </label>
    </div>
  </section>
{/if}
