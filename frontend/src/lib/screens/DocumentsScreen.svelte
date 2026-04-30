<script lang="ts">
  import { documentsStore } from "../stores/documents";
  import type { DocumentKind } from "../types";

  const documents = documentsStore;

  let createKind: DocumentKind = "act";

  const chainTargetLabels: Record<DocumentKind, string> = {
    invoice: "Рахунок",
    act: "Акт",
    waybill: "Накладна"
  };

  $: createCounterpartyId = $documents.draftContext?.counterpartyId ?? "";

  function onDocumentSearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void documents.load(input.value);
  }

  function onCreateDraft() {
    void documents.create(createCounterpartyId, createKind);
  }

  function onEditorNumberChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    documents.updateFormField("number", input.value);
  }

  function onEditorDateChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    documents.updateFormField("date", input.value);
  }

  function onEditorNotesChange(event: Event) {
    const input = event.currentTarget as HTMLTextAreaElement;
    documents.updateFormField("notes", input.value);
  }

  function onItemFieldChange(
    index: number,
    field: "description" | "unit" | "quantity" | "price",
    event: Event
  ) {
    const input = event.currentTarget as HTMLInputElement;
    documents.updateItemField(index, field, input.value);
  }

  function onDeleteCurrent() {
    void documents.deleteCurrent();
  }

  function onCreateChainDraft(kind: DocumentKind) {
    void documents.createChainDraft(kind);
  }

  function onReloadChain() {
    void documents.reloadCurrent();
  }

  function getChainTargets(kind: DocumentKind): DocumentKind[] {
    if (kind === "invoice") {
      return ["act", "waybill"];
    }
    if (kind === "act") {
      return ["waybill"];
    }
    return [];
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Документи</h2>
      <p>{$documents.list?.totalCount ?? 0} документів</p>
    </div>
    <input placeholder="Пошук документів" on:input={onDocumentSearch} />
  </div>

  <div class="create-strip">
    <input bind:value={createCounterpartyId} placeholder="UUID контрагента для нового документа" />
    <select bind:value={createKind}>
      <option value="act">Акт</option>
      <option value="invoice">Рахунок</option>
      <option value="waybill">Накладна</option>
    </select>
    <button on:click={onCreateDraft}>Створити чернетку</button>
  </div>

  {#if $documents.draftContext?.counterpartyName}
    <p class="hint">Поточний create context: {$documents.draftContext.counterpartyName}</p>
  {/if}

  {#if $documents.message}
    <p class="message">{$documents.message}</p>
  {/if}

  {#if $documents.error}
    <p class="error">{$documents.error}</p>
  {/if}

  <div class="documents-list">
    {#each $documents.list?.items ?? [] as item}
      <button class="doc-row" on:click={() => documents.open(item.id)}>
        <div>
          <strong>{item.number}</strong>
          <p>{item.counterparty}</p>
        </div>
        <div>
          <span class="doc-kind">{item.kind}</span>
          <span>{item.amountStr}</span>
          <span>{item.statusLabel}</span>
        </div>
      </button>
    {/each}
  </div>
</section>

{#if $documents.editor}
  <section class="editor-sheet">
    <div class="editor-header">
      <div>
        <h3>{$documents.editor.form.title}</h3>
        <p>{$documents.editor.form.counterpartyName}</p>
      </div>
      <div class="editor-actions">
        <button on:click={() => documents.addItem()}>Додати позицію</button>
        <button on:click={() => documents.save()}>Зберегти</button>
        <button on:click={() => documents.advanceStatus()}>Наступний статус</button>
        <button class="ghost-danger" on:click={onDeleteCurrent}>Видалити</button>
        <button on:click={() => documents.closeEditor()}>Закрити</button>
      </div>
    </div>

    <div class="editor-grid">
      <label>
        Номер
        <input value={$documents.editor.form.number} on:input={onEditorNumberChange} />
      </label>
      <label>
        Дата
        <input value={$documents.editor.form.date} on:input={onEditorDateChange} />
      </label>
      <label class="editor-grid-span">
        Примітки
        <textarea rows="3" value={$documents.editor.form.notes} on:input={onEditorNotesChange}></textarea>
      </label>
    </div>

    <div class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>Ланцюжок документа</strong>
          <p>Створення похідних документів і швидке оновлення зв'язків.</p>
        </div>
        <div class="chain-actions">
          <button on:click={onReloadChain}>Оновити</button>
          {#each getChainTargets($documents.editor.form.kind) as targetKind}
            <button on:click={() => onCreateChainDraft(targetKind)}>+ {chainTargetLabels[targetKind]}</button>
          {/each}
        </div>
      </div>

      {#if $documents.chain}
        <div class="chain-steps">
          {#each $documents.chain.steps as step}
            <div class:missing={!step.exists} class="chain-step">
              <div>
                <strong>{step.docType}</strong>
                <p>{step.docNumber}</p>
              </div>
              <div>
                <span>{step.amountStr}</span>
                <span>{step.status}</span>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="editor-items">
      {#each $documents.editor.items as item, index}
        <div class="editor-item">
          <input value={item.description} placeholder="Опис" on:input={(event) => onItemFieldChange(index, "description", event)} />
          <input value={item.unit} placeholder="Од." on:input={(event) => onItemFieldChange(index, "unit", event)} />
          <input value={item.quantity} placeholder="Кількість" on:input={(event) => onItemFieldChange(index, "quantity", event)} />
          <input value={item.price} placeholder="Ціна" on:input={(event) => onItemFieldChange(index, "price", event)} />
          <button on:click={() => documents.removeItem(index)}>Видалити</button>
        </div>
      {/each}
    </div>
  </section>
{/if}
