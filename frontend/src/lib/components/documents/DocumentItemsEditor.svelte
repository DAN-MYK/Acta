<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { DOCUMENTS_COPY, formatDocumentItemsLabel } from "../../config/ui";
  import { formatDocumentDraftTotal, formatDocumentItemTotal } from "../../documentMoney";
  import type { DocumentDraftItemDto } from "../../types";

  type ItemField = "description" | "unit" | "quantity" | "price";

  export let items: DocumentDraftItemDto[] = [];
  export let loading = false;

  const dispatch = createEventDispatcher<{
    addItem: void;
    removeItem: number;
    updateItemField: { index: number; field: ItemField; value: string };
  }>();

  function onItemFieldInput(index: number, field: ItemField, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    dispatch("updateItemField", { index, field, value: input.value });
  }
</script>

<div class="editor-items-card">
  <div class="editor-items-header">
    <strong>Позиції документа</strong>
    <div class="editor-items-summary">
      <span class="editor-items-count">{formatDocumentItemsLabel(items.length)}</span>
      <strong>{formatDocumentDraftTotal(items)}</strong>
      <button class="btn-secondary" on:click={() => dispatch("addItem")} disabled={loading}>
        Додати позицію
      </button>
    </div>
  </div>

  <div class="editor-items">
    {#if items.length === 0}
      <div class="editor-items-empty" data-testid="documents-items-empty">
        <strong>{DOCUMENTS_COPY.itemsEmptyTitle}</strong>
        <p>{DOCUMENTS_COPY.itemsEmptyDescription}</p>
        <button class="btn-primary" on:click={() => dispatch("addItem")} disabled={loading}>
          Додати першу позицію
        </button>
      </div>
    {:else}
      <div class="editor-item editor-item-head">
        <span>Опис</span>
        <span>Од.</span>
        <span class="editor-item-cell-numeric">Кількість</span>
        <span class="editor-item-cell-numeric">Ціна, грн</span>
        <span class="editor-item-cell-numeric">Сума</span>
        <span></span>
      </div>
      {#each items as item, index}
        <div class="editor-item">
          <input
            aria-label={`Опис рядка ${index + 1}`}
            value={item.description}
            placeholder="Опишіть товар або послугу"
            on:input={(event) => onItemFieldInput(index, "description", event)}
            disabled={loading}
          />
          <input
            aria-label={`Одиниця рядка ${index + 1}`}
            value={item.unit}
            placeholder="шт / год"
            on:input={(event) => onItemFieldInput(index, "unit", event)}
            disabled={loading}
          />
          <input
            aria-label={`Кількість рядка ${index + 1}`}
            class="editor-item-cell-numeric"
            value={item.quantity}
            placeholder="0"
            inputmode="decimal"
            on:input={(event) => onItemFieldInput(index, "quantity", event)}
            disabled={loading}
          />
          <input
            aria-label={`Ціна рядка ${index + 1}`}
            class="editor-item-cell-numeric"
            value={item.price}
            placeholder="0,00"
            inputmode="decimal"
            on:input={(event) => onItemFieldInput(index, "price", event)}
            disabled={loading}
          />
          <span
            class="editor-item-cell-numeric editor-item-sum"
            aria-label={`Сума рядка ${index + 1}`}
          >{formatDocumentItemTotal(item.quantity, item.price)}</span>
          <button
            class="btn-icon-danger editor-item-remove"
            aria-label={`Прибрати рядок ${index + 1}`}
            on:click={() => dispatch("removeItem", index)}
            disabled={loading}
          >
            ×
          </button>
        </div>
      {/each}
    {/if}
  </div>
</div>
