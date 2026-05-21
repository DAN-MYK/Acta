<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import AppIcon from "../AppIcon.svelte";
  import {
    EDITOR_DIRTY_COPY,
    DOCUMENTS_COPY,
    DOCUMENT_DIRECTION_OPTIONS,
    DOCUMENT_KIND_META,
    getDocumentChainTargets,
    resolveDocumentKindMeta,
    supportsDocumentPdfGeneration
  } from "../../config/ui";
  import type { DocumentChainDto, DocumentEditorDto, DocumentKind } from "../../types";

  type CounterpartyOption = {
    id: string;
    name: string;
  };

  type FormField = "direction" | "number" | "date" | "counterpartyId" | "notes";

  export let editor: DocumentEditorDto;
  export let chain: DocumentChainDto | null = null;
  export let pendingNew = false;
  export let loading = false;
  export let companyName = "";
  export let counterparties: CounterpartyOption[] = [];
  export let pendingDirtyClose = false;
  export let pendingDelete = false;
  export let chainMenuOpen = false;
  export let isReassigning = false;
  export let reassignTargetId = "";
  export let sectionElement: HTMLElement | null = null;

  const dispatch = createEventDispatcher<{
    close: void;
    cancelDiscardChanges: void;
    confirmDiscardChanges: void;
    cancelDelete: void;
    confirmDelete: void;
    save: void;
    toggleChainMenu: void;
    advanceStatus: void;
    createChainDraft: DocumentKind;
    createAdjustmentAct: void;
    generatePdf: void;
    deleteCurrent: void;
    updateFormField: { field: FormField; value: string };
    openCpCreate: void;
    selectCounterparty: string;
    openCpEdit: string;
    changeCounterparty: string;
  }>();

  const documentKindMeta = DOCUMENT_KIND_META;

  $: currentChainStatus = (() => {
    const steps = chain?.steps ?? [];
    return steps.length > 0 ? steps[steps.length - 1].status : "Чернетка";
  })();

  $: editorKindMeta = resolveDocumentKindMeta(editor.form.kind);

  function updateFormField(field: FormField, value: string) {
    dispatch("updateFormField", { field, value });
  }

  function onCounterpartyChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    const id = select.value;
    if (id === "__new__") {
      select.value = "";
      dispatch("openCpCreate");
      return;
    }

    dispatch("selectCounterparty", id);
  }

  function onNumberInput(event: Event) {
    updateFormField("number", (event.currentTarget as HTMLInputElement).value);
  }

  function onDateInput(event: Event) {
    updateFormField("date", (event.currentTarget as HTMLInputElement).value);
  }

  function onNotesInput(event: Event) {
    updateFormField("notes", (event.currentTarget as HTMLTextAreaElement).value);
  }
</script>

<button
  type="button"
  class="documents-drawer-backdrop"
  aria-label="Закрити редактор"
  data-testid="documents-drawer-backdrop"
  on:click={() => dispatch("close")}
></button>

<section
  class="editor-sheet documents-drawer"
  bind:this={sectionElement}
  role="dialog"
  aria-modal="true"
  aria-labelledby="documents-drawer-title"
  data-testid="documents-drawer"
>
  {#if pendingDirtyClose}
    <div
      class="editor-dirty-banner"
      role="alertdialog"
      aria-live="assertive"
      aria-labelledby="documents-dirty-banner-title"
      data-testid="documents-dirty-banner"
    >
      <div>
        <strong id="documents-dirty-banner-title">{EDITOR_DIRTY_COPY.dirtyTitle}</strong>
        <p>{EDITOR_DIRTY_COPY.dirtyDescription}</p>
      </div>
      <div class="editor-dirty-actions">
        <button
          type="button"
          class="btn-ghost btn-sm"
          on:click={() => dispatch("cancelDiscardChanges")}
          data-testid="documents-dirty-banner-cancel"
        >
          {EDITOR_DIRTY_COPY.dirtyStay}
        </button>
        <button
          type="button"
          class="btn-danger btn-sm"
          on:click={() => dispatch("confirmDiscardChanges")}
          data-testid="documents-dirty-banner-discard"
        >
          {EDITOR_DIRTY_COPY.dirtyDiscard}
        </button>
      </div>
    </div>
  {/if}

  {#if pendingDelete}
    <div
      class="confirm-delete-banner"
      role="alertdialog"
      aria-live="assertive"
      aria-labelledby="documents-confirm-delete-title"
      data-testid="documents-confirm-delete-banner"
    >
      <div>
        <strong id="documents-confirm-delete-title">Видалити документ?</strong>
        <p>{DOCUMENTS_COPY.confirmDeleteCurrent}</p>
      </div>
      <div class="editor-dirty-actions">
        <button type="button" class="btn-ghost btn-sm" on:click={() => dispatch("cancelDelete")}>Скасувати</button>
        <button
          type="button"
          class="btn-danger btn-sm"
          on:click={() => dispatch("confirmDelete")}
          data-testid="documents-confirm-delete-confirm"
        >
          Видалити
        </button>
      </div>
    </div>
  {/if}

  <div class="editor-header">
    <div>
      <div class="editor-header-meta">
        <span class="doc-kind-badge">
          <AppIcon name={editorKindMeta.icon} size={14} />
          <span>{editorKindMeta.label}</span>
        </span>
        <span class="doc-status-chip">{currentChainStatus}</span>
      </div>
      <h3 id="documents-drawer-title" tabindex="-1">{editor.form.title}</h3>
    </div>
    <div class="editor-actions">
      <button
        class="btn-primary"
        on:click={() => dispatch("save")}
        disabled={loading}
        aria-busy={loading ? "true" : "false"}
      >
        Зберегти
      </button>

      <div class="chain-menu" class:chain-menu-open={chainMenuOpen}>
        <button
          class="btn-secondary chain-menu-trigger"
          type="button"
          aria-haspopup="menu"
          aria-expanded={chainMenuOpen}
          on:click|stopPropagation={() => dispatch("toggleChainMenu")}
          disabled={loading}
        >
          <span>Дії далі</span>
          <span aria-hidden="true" class="chain-menu-caret">▾</span>
        </button>
        {#if chainMenuOpen}
          <div
            class="chain-menu-popover"
            role="menu"
          >
            <button
              role="menuitem"
              type="button"
              class="chain-menu-item"
              on:click={() => dispatch("advanceStatus")}
              disabled={loading}
            >
              Наступний статус
            </button>
            {#each getDocumentChainTargets(editor.form.kind) as targetKind}
              <button
                role="menuitem"
                type="button"
                class="chain-menu-item"
                data-testid={`documents-chain-create-${targetKind}`}
                on:click={() => dispatch("createChainDraft", targetKind)}
                disabled={loading}
              >
                <AppIcon name={documentKindMeta[targetKind].icon} size={16} />
                <span>Створити {documentKindMeta[targetKind].actionLabel}</span>
              </button>
            {/each}
            {#if editor.form.kind === "act" && !pendingNew}
              <button
                role="menuitem"
                type="button"
                class="chain-menu-item"
                data-testid="documents-chain-create-adjustment-act"
                on:click={() => dispatch("createAdjustmentAct")}
                disabled={loading}
              >
                <AppIcon name="act" size={16} />
                <span>+ Коригування</span>
              </button>
            {/if}
          </div>
        {/if}
      </div>

      {#if supportsDocumentPdfGeneration(editor.form.kind)}
        <button class="btn-ghost" on:click={() => dispatch("generatePdf")} disabled={loading}>
          PDF
        </button>
      {/if}
      <div class="editor-actions-close">
        <button class="btn-danger" on:click={() => dispatch("deleteCurrent")} disabled={loading} data-testid="documents-delete-current-btn">
          Видалити
        </button>
        <button class="btn-ghost" on:click={() => dispatch("close")} disabled={loading}>
          Закрити
        </button>
      </div>
    </div>
  </div>

  <div class="editor-grid">
    <div class="editor-field-readonly editor-grid-span">
      <span class="editor-field-readonly-label">Компанія</span>
      <span class="editor-field-readonly-value">{companyName}</span>
      <span class="editor-field-readonly-hint">тільки перегляд</span>
    </div>

    {#if editor.form.kind === "adjustment_act"}
      <div class="editor-field-readonly editor-grid-span">
        <span class="editor-field-readonly-label">Напрямок</span>
        <span class="editor-field-readonly-value">
          {editor.form.direction === "outgoing" ? DOCUMENT_DIRECTION_OPTIONS[0].label : DOCUMENT_DIRECTION_OPTIONS[1].label}
        </span>
        <span class="editor-field-readonly-hint">тільки перегляд</span>
      </div>
      {#if editor.form.originalActNumber}
        <div class="editor-field-readonly editor-grid-span">
          <span class="editor-field-readonly-label">До акту</span>
          <span class="editor-field-readonly-value">{editor.form.originalActNumber}</span>
          <span class="editor-field-readonly-hint">оригінальний акт</span>
        </div>
      {/if}
    {:else}
      <fieldset class="editor-direction-fieldset editor-grid-span">
        <legend>Напрямок</legend>
        <label class="editor-direction-option">
          <input
            type="radio"
            name="direction"
            value="outgoing"
            checked={editor.form.direction === "outgoing"}
            on:change={() => updateFormField("direction", "outgoing")}
            disabled={loading}
          />
          {DOCUMENT_DIRECTION_OPTIONS[0].label}
        </label>
        <label class="editor-direction-option">
          <input
            type="radio"
            name="direction"
            value="incoming"
            checked={editor.form.direction === "incoming"}
            on:change={() => updateFormField("direction", "incoming")}
            disabled={loading}
          />
          {DOCUMENT_DIRECTION_OPTIONS[1].label}
        </label>
      </fieldset>
    {/if}

    <label>
      Номер
      <input
        value={editor.form.number}
        on:input={onNumberInput}
        disabled={loading}
        placeholder="Буде згенеровано автоматично"
      />
    </label>
    <label class="editor-date-field">
      Дата
      <input
        type="date"
        value={editor.form.date}
        on:input={onDateInput}
        disabled={loading}
      />
    </label>

    {#if pendingNew}
      <label class="editor-grid-span">
        Контрагент
        <select
          value={editor.form.counterpartyId}
          on:change={onCounterpartyChange}
          disabled={loading}
          required
        >
          <option value="">— Оберіть контрагента —</option>
          {#each counterparties as cp}
            <option value={cp.id}>{cp.name}</option>
          {/each}
          <option value="__new__">+ Новий контрагент...</option>
        </select>
      </label>
    {:else if isReassigning}
      <div class="editor-field-readonly editor-grid-span">
        <span class="editor-field-readonly-label">Контрагент</span>
        <div class="cp-reassign-row">
          <select bind:value={reassignTargetId} disabled={loading}>
            {#each counterparties as cp}
              <option value={cp.id}>{cp.name}</option>
            {/each}
          </select>
          <button
            class="btn-primary"
            disabled={loading || !reassignTargetId || reassignTargetId === editor.form.counterpartyId}
            on:click={() => dispatch("changeCounterparty", reassignTargetId)}
          >
            Зберегти
          </button>
          <button class="btn-ghost" on:click={() => { isReassigning = false; }}>
            Скасувати
          </button>
        </div>
      </div>
    {:else}
      <div class="editor-field-readonly editor-grid-span">
        <span class="editor-field-readonly-label">Контрагент</span>
        <span class="editor-field-readonly-value">{editor.form.counterpartyName}</span>
        <div class="cp-actions">
          <button
            class="btn-ghost btn-sm"
            on:click={() => dispatch("openCpEdit", editor.form.counterpartyId)}
            disabled={loading}
          >
            Редагувати
          </button>
          {#if editor.form.kind !== "adjustment_act"}
            <button
              class="btn-ghost btn-sm"
              on:click={() => { isReassigning = true; reassignTargetId = editor.form.counterpartyId; }}
              disabled={loading}
            >
              Змінити
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  <label class="editor-notes-field">
    Примітки
    <textarea
      rows="3"
      value={editor.form.notes}
      on:input={onNotesInput}
      disabled={loading}
    ></textarea>
  </label>

  <slot />
</section>

<style>
  .editor-direction-fieldset {
    border: 1px solid var(--acta-color-border);
    border-radius: 6px;
    padding: 8px 12px;
  }

  .editor-direction-fieldset legend {
    font-size: 12px;
    color: var(--acta-color-text-muted);
    padding: 0 4px;
  }

  .editor-direction-option {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-right: 16px;
    cursor: pointer;
  }

  .cp-reassign-row {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 4px;
  }

  .cp-reassign-row select {
    flex: 1;
  }

  .cp-actions {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }
</style>
