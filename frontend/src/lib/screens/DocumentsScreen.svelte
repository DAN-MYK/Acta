<script lang="ts">
  import { tick } from "svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import {
    DOCUMENTS_COPY,
    DOCUMENT_KIND_META,
    DOCUMENT_KIND_OPTIONS,
    formatDocumentItemsLabel,
    resolveDocumentKindMeta
  } from "../config/ui";
  import { documentsStore } from "../stores/documents";
  import { counterpartiesStore } from "../stores/counterparties";
  import { formatDocumentDraftTotal, formatDocumentItemTotal } from "../documentMoney";
  import { isFormattedMoneyNegative } from "../money";
  import type { DocumentDraftItemDto, DocumentKind } from "../types";

  const documents = documentsStore;
  const counterparties = counterpartiesStore;

  let createCounterpartyId = "";
  let createKind: DocumentKind = "act";
  let selectedCounterpartyName = "";
  let lastDraftContextCounterpartyId = "";
  let createButton: HTMLButtonElement | null = null;
  let createCounterpartySelect: HTMLSelectElement | null = null;
  let pdfFindText = "";
  let pdfReplaceText = "";
  let lastPdfDocumentId = "";
  let drawerSection: HTMLElement | null = null;
  let drawerReturnFocus: HTMLElement | null = null;
  let lastEditorDocumentId = "";
  let chainMenuOpen = false;
  let chainMenuButton: HTMLButtonElement | null = null;
  let chainMenuPopover: HTMLElement | null = null;
  let pendingDirtyClose = false;
  let panelElement: HTMLElement | null = null;

  $: {
    const nextDraftContextCounterpartyId = $documents.draftContext?.counterpartyId ?? "";

    if (nextDraftContextCounterpartyId && nextDraftContextCounterpartyId !== lastDraftContextCounterpartyId) {
      createCounterpartyId = nextDraftContextCounterpartyId;
      lastDraftContextCounterpartyId = nextDraftContextCounterpartyId;
    }

    if (!nextDraftContextCounterpartyId && lastDraftContextCounterpartyId) {
      lastDraftContextCounterpartyId = "";
    }
  }

  $: selectedCounterpartyName =
    ($counterparties.screen?.items ?? []).find((cp) => cp.id === createCounterpartyId)?.name ??
    $documents.draftContext?.counterpartyName ??
    "";

  $: {
    const currentDocumentId = $documents.editor?.form.id ?? "";
    if (currentDocumentId !== lastPdfDocumentId) {
      lastPdfDocumentId = currentDocumentId;
      pdfFindText = "";
      pdfReplaceText = "";
    }
  }

  $: {
    const editorDocumentId = $documents.editor?.form.id ?? "";

    if (editorDocumentId && !lastEditorDocumentId) {
      const previouslyFocused = document.activeElement;
      drawerReturnFocus = previouslyFocused instanceof HTMLElement ? previouslyFocused : null;
      void tick().then(() => {
        const heading = drawerSection?.querySelector<HTMLElement>("h3");
        heading?.focus();
      });
    }

    if (!editorDocumentId && lastEditorDocumentId) {
      const target = drawerReturnFocus && document.contains(drawerReturnFocus) ? drawerReturnFocus : null;
      void tick().then(() => target?.focus());
      drawerReturnFocus = null;
    }

    lastEditorDocumentId = editorDocumentId;
  }

  function onDrawerKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape") {
      return;
    }

    if (chainMenuOpen) {
      event.preventDefault();
      chainMenuOpen = false;
      chainMenuButton?.focus();
      return;
    }

    if ($documents.editor) {
      event.preventDefault();
      requestCloseDrawer();
    }
  }

  function closeEditor(force = false) {
    const result = documents.closeEditor(force);
    if (result && result.ok === false && result.reason === "dirty") {
      pendingDirtyClose = true;
      return result;
    }

    pendingDirtyClose = false;
    return result;
  }

  function confirmCloseIfDirty() {
    closeEditor();
  }

  function onDrawerBackdropClick() {
    confirmCloseIfDirty();
  }

  function requestCloseDrawer() {
    confirmCloseIfDirty();
  }

  function confirmDiscardChanges() {
    closeEditor(true);
  }

  function cancelDiscardChanges() {
    pendingDirtyClose = false;
  }

  function toggleChainMenu() {
    chainMenuOpen = !chainMenuOpen;
  }

  function closeChainMenu() {
    chainMenuOpen = false;
  }

  function onWindowClickForChainMenu(event: MouseEvent) {
    if (!chainMenuOpen) {
      return;
    }
    const target = event.target as Node | null;
    if (target && chainMenuButton?.contains(target)) {
      return;
    }
    if (target && chainMenuPopover?.contains(target)) {
      return;
    }
    closeChainMenu();
  }

  function onChainMenuAdvanceStatus() {
    void documents.advanceStatus();
    closeChainMenu();
  }

  function onChainMenuCreateChain(kind: DocumentKind) {
    onCreateChainDraft(kind);
    closeChainMenu();
  }

  const documentKindMeta = DOCUMENT_KIND_META;

  $: {
    const editorDocId = $documents.editor?.form.id ?? "";
    if (!editorDocId && chainMenuOpen) {
      chainMenuOpen = false;
    }
  }

  $: if (!$documents.editor && pendingDirtyClose) {
    pendingDirtyClose = false;
  }

  $: if (panelElement) {
    if ($documents.editor) {
      panelElement.setAttribute("inert", "");
      panelElement.setAttribute("aria-hidden", "true");
    } else {
      panelElement.removeAttribute("inert");
      panelElement.removeAttribute("aria-hidden");
    }
  }

  function onDocumentSearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void documents.load(input.value);
  }

  function onCreateDraft() {
    void documents.create(createCounterpartyId, createKind);
  }

  function focusCreateButton() {
    if (!createCounterpartyId) {
      createCounterpartySelect?.focus();
      return;
    }

    createButton?.focus();
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
    if (!window.confirm(DOCUMENTS_COPY.confirmDeleteCurrent)) {
      return;
    }

    void documents.deleteCurrent();
  }

  function onCreateChainDraft(kind: DocumentKind) {
    void documents.createChainDraft(kind);
  }

  function onAttachExistingPdf() {
    void documents.attachExistingPdf();
  }

  function onOpenCurrentPdf() {
    void documents.openCurrentPdf();
  }

  function onApplyPdfTextReplace() {
    void documents.applyPdfTextReplace(pdfFindText, pdfReplaceText);
  }

  function onToggleSelection(docId: string) {
    documents.toggleSelected(docId);
  }

  function onToggleSelectAll() {
    documents.selectAllVisible();
  }

  function onBulkDelete() {
    if (!window.confirm(DOCUMENTS_COPY.confirmDeleteBulk)) {
      return;
    }

    void documents.bulkDelete();
  }

  function onBulkAdvanceStatus() {
    void documents.bulkAdvanceStatus();
  }

  function getChainTargets(kind: string): DocumentKind[] {
    if (kind === "invoice") {
      return ["act", "waybill"];
    }
    if (kind === "act") {
      return ["waybill"];
    }
    return [];
  }

  function getDocumentKindLabel(kind: string): string {
    return resolveDocumentKindMeta(kind).label;
  }

  const directionLabels: Record<string, string> = {
    outgoing: "↑ Вихідний",
    incoming: "↓ Вхідний"
  };

  function getCreateButtonLabel(kind: DocumentKind): string {
    const tab = $documents.activeTab;
    const dirSuffix = tab === "incoming" ? " (вхідний)" : tab === "outgoing" ? " (вихідний)" : "";
    if (kind === "invoice") return `Створити рахунок${dirSuffix}`;
    if (kind === "waybill") return `Створити накладну${dirSuffix}`;
    return `Створити акт${dirSuffix}`;
  }

  function getItemsCountLabel(count: number): string {
    return formatDocumentItemsLabel(count);
  }

  function getCurrentChainStatus() {
    const steps = $documents.chain?.steps ?? [];
    return steps.length > 0 ? steps[steps.length - 1].status : "Чернетка";
  }

  function getEditorKindIcon(kind: string) {
    return resolveDocumentKindMeta(kind).icon;
  }

  function supportsExistingPdfFlow(kind: string): boolean {
    return kind === "invoice" || kind === "waybill";
  }
</script>

<section
  bind:this={panelElement}
  class="panel"
  data-testid="documents-screen"
>
  <div class="panel-header">
    <div>
      <h2>Документи</h2>
      <p>{$documents.list?.totalCount ?? 0} документів</p>
    </div>
    <input
      placeholder="Пошук документів"
      on:input={onDocumentSearch}
      disabled={$documents.loading}
      aria-busy={$documents.loading ? "true" : "false"}
    />
  </div>

  <div class="documents-nav-tabs" role="tablist" aria-label="Напрямок документів">
    {#each [
      { value: "all",      label: "Всі" },
      { value: "outgoing", label: "Вихідні" },
      { value: "incoming", label: "Вхідні" }
    ] as tab}
      <button
        role="tab"
        type="button"
        class="nav-tab"
        class:nav-tab-active={$documents.activeTab === tab.value}
        on:click={() => documents.setTab(tab.value as "all" | "outgoing" | "incoming")}
        disabled={$documents.loading}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <div class="documents-kind-chips" role="group" aria-label="Тип документа">
    {#each [
      { value: null,       label: "Всі" },
      { value: "act",      label: "Акти" },
      { value: "invoice",  label: "Рахунки" },
      { value: "waybill",  label: "Накладні" }
    ] as chip}
      <button
        type="button"
        class="kind-chip"
        class:kind-chip-active={$documents.kindFilter === chip.value}
        on:click={() => documents.setKindFilter(chip.value as DocumentKind | null)}
        disabled={$documents.loading}
      >
        {chip.label}
      </button>
    {/each}
  </div>

  <div class="documents-create-bar" data-testid="documents-create-strip">
    <select
      bind:this={createCounterpartySelect}
      bind:value={createCounterpartyId}
      disabled={$documents.loading}
      aria-label="Контрагент"
    >
      <option value="">— Оберіть контрагента —</option>
      {#each $counterparties.screen?.items ?? [] as cp}
        <option value={cp.id}>{cp.name}</option>
      {/each}
    </select>
    <select bind:value={createKind} disabled={$documents.loading} aria-label="Тип документа">
      {#each DOCUMENT_KIND_OPTIONS as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
    <button
      bind:this={createButton}
      class="btn-primary"
      data-testid="documents-create-button"
      type="button"
      disabled={!createCounterpartyId || $documents.loading}
      on:click={onCreateDraft}
      aria-busy={$documents.loading ? "true" : "false"}
    >
      <AppIcon name={documentKindMeta[createKind].icon} surface={true} />
      <span>{getCreateButtonLabel(createKind)}</span>
    </button>
  </div>

  <div class="bulk-actions">
    <label class="bulk-select-all">
      <input
        type="checkbox"
        checked={
          ($documents.list?.items.length ?? 0) > 0 &&
          ($documents.list?.items ?? []).every((item) => $documents.selectedIds.includes(item.id))
        }
        on:click|stopPropagation={onToggleSelectAll}
      />
      <span>Вибрати все</span>
    </label>

    <button
      class="btn-secondary"
      disabled={$documents.selectedIds.length === 0 || $documents.loading}
      on:click={onBulkAdvanceStatus}
    >
      Оновити статус вибраних
    </button>

    <button
      class="btn-danger"
      disabled={$documents.selectedIds.length === 0 || $documents.loading}
      on:click={onBulkDelete}
    >
      Видалити вибрані
    </button>
  </div>

  {#if $documents.message}
    <p class="message" role="status" aria-live="polite">{$documents.message}</p>
  {/if}

  {#if $documents.error}
    <div class="status-banner is-error" role="alert" aria-live="assertive">
      <div>
        <strong>Потрібна увага</strong>
        <p>{$documents.error}</p>
      </div>
    </div>
  {/if}

  {#if $documents.initialLoading}
    <SkeletonRow count={5} />
  {:else if ($documents.list?.items.length ?? 0) === 0}
    <div class="empty-state-card" data-testid="documents-empty-state">
      <span class="empty-state-eyebrow">Почніть зі сценарію</span>
      <strong>{DOCUMENTS_COPY.emptyTitle}</strong>
      <p>{DOCUMENTS_COPY.emptyDescription}</p>
      <div class="empty-state-actions">
        <button
          class="btn-primary"
          type="button"
          data-testid="documents-empty-primary-action"
          on:click={focusCreateButton}
        >
          {DOCUMENTS_COPY.emptyAction}
        </button>
      </div>
    </div>
  {:else}
    <div class="documents-list" data-testid="documents-list">
      {#each $documents.list?.items ?? [] as item}
        <div class="doc-row doc-row-rich" data-testid={`documents-row-${item.id}`}>
          <label class="doc-row-checkbox" aria-label={`Вибрати ${item.number}`}>
            <input
              type="checkbox"
              checked={$documents.selectedIds.includes(item.id)}
              on:click|stopPropagation={() => onToggleSelection(item.id)}
            />
          </label>

          <button
            class="doc-row-open"
            type="button"
            on:click={() => documents.open(item.id)}
            disabled={$documents.loading}
            aria-label={`Відкрити документ ${item.number}`}
          >
            <div class="doc-row-body">
              <div>
                <strong class="doc-row-title">
                  <AppIcon name={resolveDocumentKindMeta(item.kind).icon} surface={true} size={16} />
                  <span>{item.number}</span>
                </strong>
                <p>{item.counterparty}</p>
              </div>
              <div class="doc-row-meta">
                <span>{item.date}</span>
                <span class="money-value" data-negative={isFormattedMoneyNegative(item.amountStr)}>{item.amountStr}</span>
                <span class="doc-kind-badge">
                  <AppIcon name={resolveDocumentKindMeta(item.kind).icon} size={14} />
                  <span>{getDocumentKindLabel(item.kind)}</span>
                </span>
                <span class="doc-status-chip">{item.statusLabel}</span>
                <span class="doc-direction-badge" data-direction={item.direction}>
                  {directionLabels[item.direction] ?? item.direction}
                </span>
              </div>
            </div>
          </button>
        </div>
      {/each}
    </div>
  {/if}
</section>

<svelte:window on:keydown={onDrawerKeydown} on:click={onWindowClickForChainMenu} />

{#if $documents.editor}
  <button
    type="button"
    class="documents-drawer-backdrop"
    aria-label="Закрити редактор"
    data-testid="documents-drawer-backdrop"
    on:click={onDrawerBackdropClick}
  ></button>
  <section
    class="editor-sheet documents-drawer"
    bind:this={drawerSection}
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
          <strong id="documents-dirty-banner-title">{DOCUMENTS_COPY.dirtyTitle}</strong>
          <p>{DOCUMENTS_COPY.dirtyDescription}</p>
        </div>
        <div class="editor-dirty-actions">
          <button
            type="button"
            class="btn-ghost btn-sm"
            on:click={cancelDiscardChanges}
            data-testid="documents-dirty-banner-cancel"
          >
            {DOCUMENTS_COPY.dirtyStay}
          </button>
          <button
            type="button"
            class="btn-danger btn-sm"
            on:click={confirmDiscardChanges}
            data-testid="documents-dirty-banner-discard"
          >
            {DOCUMENTS_COPY.dirtyDiscard}
          </button>
        </div>
      </div>
    {/if}
    <div class="editor-header">
      <div>
        <div class="editor-header-meta">
          <span class="doc-kind-badge">
            <AppIcon name={getEditorKindIcon($documents.editor.form.kind)} size={14} />
            <span>{getDocumentKindLabel($documents.editor.form.kind)}</span>
          </span>
          <span class="doc-status-chip">{getCurrentChainStatus()}</span>
        </div>
        <h3 id="documents-drawer-title" tabindex="-1">{$documents.editor.form.title}</h3>
        <p>{$documents.editor.form.counterpartyName}</p>
      </div>
      <div class="editor-actions">
        <button
          class="btn-primary"
          on:click={() => documents.save()}
          disabled={$documents.loading}
          aria-busy={$documents.loading ? "true" : "false"}
        >
          Зберегти
        </button>

        <div class="chain-menu" class:chain-menu-open={chainMenuOpen}>
          <button
            bind:this={chainMenuButton}
            class="btn-secondary chain-menu-trigger"
            type="button"
            aria-haspopup="menu"
            aria-expanded={chainMenuOpen}
            on:click|stopPropagation={toggleChainMenu}
            disabled={$documents.loading}
          >
            <span>Дії далі</span>
            <span aria-hidden="true" class="chain-menu-caret">▾</span>
          </button>
          <div
            bind:this={chainMenuPopover}
            class="chain-menu-popover"
            role="menu"
            hidden={!chainMenuOpen}
          >
            <button
              role="menuitem"
              type="button"
              class="chain-menu-item"
              on:click={onChainMenuAdvanceStatus}
              disabled={$documents.loading}
            >
              Наступний статус
            </button>
            {#each getChainTargets($documents.editor.form.kind) as targetKind}
              <button
                role="menuitem"
                type="button"
                class="chain-menu-item"
                data-testid={`documents-chain-create-${targetKind}`}
                on:click={() => onChainMenuCreateChain(targetKind)}
                disabled={$documents.loading}
              >
                <AppIcon name={documentKindMeta[targetKind].icon} size={16} />
                <span>Створити {documentKindMeta[targetKind].actionLabel}</span>
              </button>
            {/each}
          </div>
        </div>

        {#if ["act", "invoice"].includes($documents.editor.form.kind)}
          <button class="btn-ghost" on:click={() => documents.generatePdf()} disabled={$documents.loading}>
            PDF
          </button>
        {/if}
        <button class="btn-danger" on:click={onDeleteCurrent} disabled={$documents.loading}>
          Видалити
        </button>
        <button class="btn-ghost" on:click={requestCloseDrawer} disabled={$documents.loading}>
          Закрити
        </button>
      </div>
    </div>

    <div class="editor-grid">
      <label>
        Номер
        <input value={$documents.editor.form.number} on:input={onEditorNumberChange} disabled={$documents.loading} />
      </label>
      <label class="editor-date-field">
        Дата
        <input
          type="date"
          value={$documents.editor.form.date}
          on:input={onEditorDateChange}
          disabled={$documents.loading}
        />
      </label>
      <label class="editor-grid-span">
        Примітки
        <textarea
          rows="3"
          value={$documents.editor.form.notes}
          on:input={onEditorNotesChange}
          disabled={$documents.loading}
        ></textarea>
      </label>
      <fieldset class="editor-direction-fieldset editor-grid-span">
        <legend>Напрямок</legend>
        <label class="editor-direction-option">
          <input
            type="radio"
            name="direction"
            value="outgoing"
            checked={$documents.editor?.form.direction === "outgoing"}
            on:change={() => documents.updateFormField("direction", "outgoing")}
            disabled={$documents.loading}
          />
          ↑ Вихідний
        </label>
        <label class="editor-direction-option">
          <input
            type="radio"
            name="direction"
            value="incoming"
            checked={$documents.editor?.form.direction === "incoming"}
            on:change={() => documents.updateFormField("direction", "incoming")}
            disabled={$documents.loading}
          />
          ↓ Вхідний
        </label>
      </fieldset>
    </div>

    <div class="editor-items-card">
      <div class="editor-items-header">
        <strong>Позиції документа</strong>
        <div class="editor-items-summary">
          <span class="editor-items-count">{getItemsCountLabel($documents.editor.items.length)}</span>
          <strong>{formatDocumentDraftTotal($documents.editor.items)}</strong>
          <button class="btn-secondary" on:click={() => documents.addItem()} disabled={$documents.loading}>
            Додати позицію
          </button>
        </div>
      </div>

      <div class="editor-items">
        {#if $documents.editor.items.length === 0}
          <div class="editor-items-empty" data-testid="documents-items-empty">
            <strong>{DOCUMENTS_COPY.itemsEmptyTitle}</strong>
            <p>{DOCUMENTS_COPY.itemsEmptyDescription}</p>
            <button class="btn-primary" on:click={() => documents.addItem()} disabled={$documents.loading}>
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
          {#each $documents.editor.items as item, index}
            <div class="editor-item">
              <input
                aria-label={`Опис рядка ${index + 1}`}
                value={item.description}
                placeholder="Опишіть товар або послугу"
                on:input={(event) => onItemFieldChange(index, "description", event)}
                disabled={$documents.loading}
              />
              <input
                aria-label={`Одиниця рядка ${index + 1}`}
                value={item.unit}
                placeholder="шт / год"
                on:input={(event) => onItemFieldChange(index, "unit", event)}
                disabled={$documents.loading}
              />
              <input
                aria-label={`Кількість рядка ${index + 1}`}
                class="editor-item-cell-numeric"
                value={item.quantity}
                placeholder="0"
                inputmode="decimal"
                on:input={(event) => onItemFieldChange(index, "quantity", event)}
                disabled={$documents.loading}
              />
              <input
                aria-label={`Ціна рядка ${index + 1}`}
                class="editor-item-cell-numeric"
                value={item.price}
                placeholder="0,00"
                inputmode="decimal"
                on:input={(event) => onItemFieldChange(index, "price", event)}
                disabled={$documents.loading}
              />
              <span
                class="editor-item-cell-numeric editor-item-sum"
                aria-label={`Сума рядка ${index + 1}`}
              >{formatDocumentItemTotal(item.quantity, item.price)}</span>
              <button
                class="btn-icon-danger editor-item-remove"
                aria-label={`Прибрати рядок ${index + 1}`}
                on:click={() => documents.removeItem(index)}
                disabled={$documents.loading}
              >
                ✕
              </button>
            </div>
          {/each}
        {/if}
      </div>
    </div>

    {#if supportsExistingPdfFlow($documents.editor.form.kind)}
      <div class="editor-items-card existing-pdf-card" data-testid="documents-existing-pdf">
        <div class="editor-items-header">
          <div>
            <strong>Існуючий PDF</strong>
            {#if $documents.editor.pdf}
              <p class="existing-pdf-status">
                {$documents.editor.pdf.filePath} · {$documents.editor.pdf.pageCount} стор. ·
                {$documents.editor.pdf.editable ? "Exact replace доступний" : "Тільки перегляд"}
              </p>
            {:else}
              <p class="existing-pdf-status">Не прив'язано</p>
            {/if}
          </div>
          <div class="editor-actions existing-pdf-actions">
            <button class="btn-secondary" on:click={onAttachExistingPdf} disabled={$documents.loading}>
              {$documents.editor.pdf ? "Прив'язати інший PDF" : "Прив'язати PDF"}
            </button>
            {#if $documents.editor.pdf}
              <button
                class="btn-ghost"
                on:click={onOpenCurrentPdf}
                disabled={$documents.loading}
              >
                Відкрити PDF
              </button>
            {/if}
          </div>
        </div>

        {#if $documents.editor.pdf}
          <details class="existing-pdf-details" open={$documents.editor.pdf.warnings.length > 0}>
            <summary>Текстовий шар і exact replace</summary>

            <p class="existing-pdf-meta">
              Текстовий шар: {$documents.editor.pdf.hasTextOps ? "Знайдено" : "Не знайдено"}
            </p>

            {#if $documents.editor.pdf.warnings.length > 0}
              <div class="existing-pdf-warnings">
                {#each $documents.editor.pdf.warnings as warning}
                  <p>{warning}</p>
                {/each}
              </div>
            {/if}

            <label class="existing-pdf-preview">
              Витягнутий текст
              <textarea rows="10" readonly value={$documents.editor.pdf.extractedText}></textarea>
            </label>

            <div class="existing-pdf-replace">
              <label>
                <span>Знайти текст</span>
                <input bind:value={pdfFindText} placeholder="Точний фрагмент з витягнутого тексту" />
              </label>
              <label>
                <span>Замінити на</span>
                <input bind:value={pdfReplaceText} placeholder="Новий текст" />
              </label>
              <button
                class="btn-primary"
                on:click={onApplyPdfTextReplace}
                disabled={
                  $documents.loading ||
                  !$documents.editor.pdf.editable ||
                  !pdfFindText.trim() ||
                  !pdfReplaceText.trim()
                }
              >
                Застосувати exact replace
              </button>
            </div>
          </details>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  .documents-nav-tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--color-border);
    padding: 0 16px;
  }

  .nav-tab {
    padding: 8px 16px;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--color-text-sub);
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }

  .nav-tab-active {
    color: var(--color-primary);
    border-bottom-color: var(--color-primary);
    font-weight: 500;
  }

  .documents-kind-chips {
    display: flex;
    gap: 6px;
    padding: 8px 16px;
  }

  .kind-chip {
    padding: 4px 12px;
    border-radius: 12px;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    cursor: pointer;
    font-size: 13px;
    color: var(--color-text-sub);
  }

  .kind-chip-active {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: #fff;
  }

  .doc-direction-badge {
    font-size: 11px;
    color: var(--color-text-muted);
  }

  .doc-direction-badge[data-direction="outgoing"] {
    color: var(--color-success);
  }

  .doc-direction-badge[data-direction="incoming"] {
    color: var(--color-warning);
  }

  .editor-direction-fieldset {
    border: 1px solid var(--color-border);
    border-radius: 6px;
    padding: 8px 12px;
  }

  .editor-direction-fieldset legend {
    font-size: 12px;
    color: var(--color-text-sub);
    padding: 0 4px;
  }

  .editor-direction-option {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-right: 16px;
    cursor: pointer;
  }
</style>
