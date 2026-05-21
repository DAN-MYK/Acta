<script lang="ts">
  import { tick } from "svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import CounterpartyModal from "../components/CounterpartyModal.svelte";
  import DocumentFilters, { type DocumentFiltersApplyDetail } from "../components/documents/DocumentFilters.svelte";
  import DocumentCreateMenu from "../components/documents/DocumentCreateMenu.svelte";
  import DocumentList from "../components/documents/DocumentList.svelte";
  import DocumentEditorDrawer from "../components/documents/DocumentEditorDrawer.svelte";
  import DocumentItemsEditor from "../components/documents/DocumentItemsEditor.svelte";
  import DocumentPdfTools from "../components/documents/DocumentPdfTools.svelte";
  import {
    DOCUMENTS_COPY,
    DOCUMENT_KIND_FILTER_OPTIONS,
    DOCUMENT_TAB_OPTIONS,
    DOCUMENT_STATUS_OPTIONS,
    DOCUMENTS_FILTER_COPY,
    supportsExistingPdfFlow
  } from "../config/ui";
  import { documentsStore } from "../stores/documents";
  import { counterpartiesStore } from "../stores/counterparties";
  import { shellStore } from "../stores/shell";
  import type { DocumentKind } from "../types";

  const documents = documentsStore;
  const counterparties = counterpartiesStore;
  const shell = shellStore;

  let createCounterpartyId = "";
  let lastDraftContextCounterpartyId = "";
  let filtersOpen = false;
  let filterButton: HTMLButtonElement | null = null;
  let filterPopover: HTMLElement | null = null;
  let createMenuOpen = false;
  let createMenuPopover: HTMLElement | null = null;
  let drawerSection: HTMLElement | null = null;
  let drawerReturnFocus: HTMLElement | null = null;
  let lastEditorDocumentId = "";
  let chainMenuOpen = false;
  let pendingDirtyClose = false;
  let panelElement: HTMLElement | null = null;

  let isReassigning = false;
  let reassignTargetId = "";

  // Reset reassign mode when counterpartyId changes (after successful changeCounterparty)
  $: if ($documents.editor?.form.counterpartyId !== undefined && !$documents.pendingNew) {
    if (!isReassigning) reassignTargetId = $documents.editor.form.counterpartyId;
  }

  $: isDirtyCpModal = (() => {
    const m = $documents.cpModal;
    if (!m?.form || !m.snapshot) return false;
    return JSON.stringify(m.form) !== JSON.stringify(m.snapshot);
  })();

  $: {
    const nextDraftContextCounterpartyId = $documents.draftContext?.counterpartyId ?? "";

    if (nextDraftContextCounterpartyId && nextDraftContextCounterpartyId !== lastDraftContextCounterpartyId) {
      createCounterpartyId = nextDraftContextCounterpartyId;
      lastDraftContextCounterpartyId = nextDraftContextCounterpartyId;
    }

    if (!nextDraftContextCounterpartyId && lastDraftContextCounterpartyId) {
      createCounterpartyId = "";
      lastDraftContextCounterpartyId = "";
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
      return;
    }

    if ($documents.editor) {
      event.preventDefault();
      requestCloseDrawer();
    }
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    // Close floating menus first — if any were open, stop here (don't close editor too).
    if (filtersOpen || createMenuOpen) {
      filtersOpen = false;
      createMenuOpen = false;
      return;
    }
    // No floating menus were open: delegate to drawer keydown handler for editor close.
    onDrawerKeydown(event);
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

  function onWindowClick(event: MouseEvent) {
    const target = event.target instanceof Node ? event.target : null;

    if (chainMenuOpen) {
      closeChainMenu();
    }

    if (filtersOpen) {
      if (target && filterButton?.contains(target)) return;
      if (target && filterPopover?.contains(target)) return;
      filtersOpen = false;
    }

    if (createMenuOpen) {
      if (target && createMenuPopover?.contains(target)) return;
      createMenuOpen = false;
    }
  }

  function onChainMenuAdvanceStatus() {
    void documents.advanceStatus();
    closeChainMenu();
  }

  function onChainMenuCreateChain(kind: DocumentKind) {
    onCreateChainDraft(kind);
    closeChainMenu();
  }

  type DrawerFormField = "direction" | "number" | "date" | "counterpartyId" | "notes";

  function onDrawerCreateChainDraft(event: CustomEvent<DocumentKind>) {
    onChainMenuCreateChain(event.detail);
  }

  function onDrawerUpdateFormField(event: CustomEvent<{ field: DrawerFormField; value: string }>) {
    documents.updateFormField(event.detail.field, event.detail.value);
  }

  function onDrawerSelectCounterparty(event: CustomEvent<string>) {
    onEditorCounterpartySelect(event.detail);
  }

  function onDrawerOpenCpEdit(event: CustomEvent<string>) {
    void documents.openCpEdit(event.detail);
  }

  async function onDrawerChangeCounterparty(event: CustomEvent<string>) {
    const documentId = $documents.editor?.form.id ?? "";
    if (!documentId) return;

    await documents.changeCounterparty(documentId, event.detail);
    isReassigning = false;
    reassignTargetId = event.detail;
  }

  $: {
    const editorDocId = $documents.editor?.form.id ?? "";
    if (!editorDocId && chainMenuOpen) {
      chainMenuOpen = false;
    }
  }

  $: if ($documents.kindFilter && createMenuOpen) {
    createMenuOpen = false;
  }

  $: if ($documents.loading) {
    filtersOpen = false;
    createMenuOpen = false;
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

  $: activeFilterCount = (() => {
    const s = $documents;
    let n = 0;
    if (s.dateFrom || s.dateTo) n++;
    if (s.statusFilter.length > 0) n++;
    if (s.counterpartyFilterId) n++;
    if (s.amountMin || s.amountMax) n++;
    if (s.overdueOnly) n++;
    return n;
  })();

  $: filterButtonLabel = activeFilterCount > 0
    ? DOCUMENTS_FILTER_COPY.filterButtonWithCount(activeFilterCount)
    : DOCUMENTS_FILTER_COPY.filterButton;

  function statusLabelOf(code: string): string {
    return DOCUMENT_STATUS_OPTIONS.find((o) => o.value === code)?.label ?? code;
  }

  function formatPeriodChip(from: string | null, to: string | null): string {
    if (from && to) return `${DOCUMENTS_FILTER_COPY.periodLabel}: ${from} – ${to}`;
    if (from)      return `${DOCUMENTS_FILTER_COPY.periodLabel}: від ${from}`;
    if (to)        return `${DOCUMENTS_FILTER_COPY.periodLabel}: до ${to}`;
    return DOCUMENTS_FILTER_COPY.periodLabel;
  }

  function onClearAllFilters() {
    documents.clearAllFilters();
  }

  function onRemovePeriodChip()       { documents.setDateRange(null, null); }
  function onRemoveStatusChip()       { documents.setStatusFilter([]); }
  function onRemoveCounterpartyChip() { documents.setCounterpartyFilter(null); }
  function onRemoveAmountChip()       { documents.setAmountRange(null, null); }
  function onRemoveOverdueChip()      { void documents.applyPreset("all"); }

  function onApplyFilters(event: CustomEvent<DocumentFiltersApplyDetail>) {
    void documents.applyFilters({
      ...event.detail
    });
    filtersOpen = false;
  }

  $: selectedCreateKind = $documents.kindFilter;

  function onCreateDraft() {
    if (!selectedCreateKind) {
      createMenuOpen = !createMenuOpen;
      return;
    }
    if (createCounterpartyId) {
      void documents.create(createCounterpartyId, selectedCreateKind);
    } else {
      documents.openNewEditor(selectedCreateKind);
    }
  }

  function onCreateDirect(event: CustomEvent<DocumentKind>) {
    if (createCounterpartyId) {
      void documents.create(createCounterpartyId, event.detail);
    } else {
      documents.openNewEditor(event.detail);
    }
  }

  function onCreateMenuKind(kind: DocumentKind) {
    if (createCounterpartyId) {
      void documents.create(createCounterpartyId, kind);
    } else {
      documents.openNewEditor(kind);
    }
    createMenuOpen = false;
  }

  function onEditorCounterpartySelect(id: string) {
    if (id === "__new__") {
      void documents.openCpCreate();
      return;
    }
    const name = ($counterparties.screen?.items ?? []).find((cp) => cp.id === id)?.name ?? "";
    documents.updateCounterparty(id, id ? name : "");
  }

  function toggleFilters() {
    filtersOpen = !filtersOpen;
  }

  let pendingDeleteKind: 'single' | null = null;

  function onDeleteCurrent() {
    pendingDeleteKind = 'single';
  }

  function confirmDelete() {
    if (pendingDeleteKind === 'single') {
      void documents.deleteCurrent();
    }
    pendingDeleteKind = null;
  }

  function cancelDelete() {
    pendingDeleteKind = null;
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

  function onToggleSelection(docId: string) {
    documents.toggleSelected(docId);
  }

  function onToggleSelectAll() {
    documents.selectAllVisible();
  }

  const navTabs = DOCUMENT_TAB_OPTIONS;

  const kindChips = DOCUMENT_KIND_FILTER_OPTIONS;

</script>

<section
  bind:this={panelElement}
  class="panel"
  data-testid="documents-screen"
>
  <div class="nav-tabs" role="tablist" aria-label="Напрямок документів">
    {#each navTabs as tab}
      <button
        role="tab"
        type="button"
        class="nav-tab"
        class:nav-tab-active={$documents.activeTab === tab.value}
        aria-selected={$documents.activeTab === tab.value}
        on:click={() => documents.setTab(tab.value)}
        disabled={$documents.loading}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <div class="documents-toolbar" data-testid="documents-toolbar">
    <div class="documents-kind-chips" role="group" aria-label="Тип документа">
      {#each kindChips as chip}
        <button
          type="button"
          class="kind-chip"
          class:kind-chip-active={$documents.kindFilter === chip.value}
          on:click={() => documents.setKindFilter(chip.value)}
          disabled={$documents.loading}
        >
          {chip.label}
        </button>
      {/each}
    </div>

    <div class="documents-toolbar-actions">
      <div class="documents-toolbar-popover-anchor">
        <button
          bind:this={filterButton}
          class="btn-secondary"
          class:filter-popover-btn-active={filtersOpen}
          data-testid="documents-filter-button"
          type="button"
          aria-expanded={filtersOpen}
          aria-controls={filtersOpen ? "documents-filter-popover" : undefined}
          on:click={toggleFilters}
          disabled={$documents.loading}
        >
          <span>{filterButtonLabel}</span>
        </button>

        {#if filtersOpen}
          <div bind:this={filterPopover}>
            <DocumentFilters
              open={filtersOpen}
              loading={$documents.loading}
              dateFrom={$documents.dateFrom}
              dateTo={$documents.dateTo}
              statusFilter={$documents.statusFilter}
              amountMin={$documents.amountMin}
              amountMax={$documents.amountMax}
              overdueOnly={$documents.overdueOnly}
              counterpartyFilterId={$documents.counterpartyFilterId}
              counterparties={$counterparties.screen?.items ?? []}
              on:apply={onApplyFilters}
              on:close={() => { filtersOpen = false; }}
            />
          </div>
        {/if}
      </div>

      {#if activeFilterCount > 0}
        <button
          class="btn-ghost"
          type="button"
          data-testid="documents-clear-filters"
          on:click={onClearAllFilters}
          disabled={$documents.loading}
        >
          {DOCUMENTS_FILTER_COPY.clearAll}
        </button>
      {/if}

      <div class="documents-toolbar-popover-anchor" bind:this={createMenuPopover}>
        <DocumentCreateMenu
          open={createMenuOpen}
          loading={$documents.loading}
          disabled={$documents.kindFilter === "adjustment_act"}
          selectedKind={selectedCreateKind}
          activeTab={$documents.activeTab}
          on:toggle={() => { createMenuOpen = !createMenuOpen; }}
          on:directCreate={onCreateDirect}
          on:menuCreate={(event) => onCreateMenuKind(event.detail)}
        />
      </div>
    </div>
  </div>

  {#if activeFilterCount > 0}
    <div class="documents-active-filters" data-testid="documents-active-filters">
      <span class="documents-active-label">{DOCUMENTS_FILTER_COPY.activeFiltersLabel}</span>

      {#if $documents.dateFrom || $documents.dateTo}
        <button class="active-chip" type="button" on:click={onRemovePeriodChip} aria-label="Прибрати фільтр період">
          <span>{formatPeriodChip($documents.dateFrom, $documents.dateTo)}</span>
          <span aria-hidden="true">×</span>
        </button>
      {/if}

      {#if $documents.statusFilter.length > 0}
        <button class="active-chip" type="button" on:click={onRemoveStatusChip} aria-label="Прибрати фільтр статус">
          <span>{DOCUMENTS_FILTER_COPY.statusLabel}: {$documents.statusFilter.map(statusLabelOf).join(", ")}</span>
          <span aria-hidden="true">×</span>
        </button>
      {/if}

      {#if $documents.counterpartyFilterId}
        <button class="active-chip" type="button" on:click={onRemoveCounterpartyChip} aria-label="Прибрати фільтр контрагент">
          <span>{DOCUMENTS_FILTER_COPY.counterpartyLabel}: {
            ($counterparties.screen?.items ?? []).find((c) => c.id === $documents.counterpartyFilterId)?.name ?? ""
          }</span>
          <span aria-hidden="true">×</span>
        </button>
      {/if}

      {#if $documents.amountMin || $documents.amountMax}
        <button class="active-chip" type="button" on:click={onRemoveAmountChip} aria-label="Прибрати фільтр сума">
          <span>{DOCUMENTS_FILTER_COPY.amountLabel}: {$documents.amountMin ?? "0"} – {$documents.amountMax ?? "∞"}</span>
          <span aria-hidden="true">×</span>
        </button>
      {/if}

      {#if $documents.overdueOnly}
        <button class="active-chip" type="button" on:click={onRemoveOverdueChip} aria-label="Прибрати фільтр прострочені">
          <span>Прострочені</span>
          <span aria-hidden="true">×</span>
        </button>
      {/if}
    </div>
  {/if}

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
          on:click={onCreateDraft}
        >
          {DOCUMENTS_COPY.emptyAction}
        </button>
      </div>
    </div>
  {:else}
    <DocumentList
      items={$documents.list?.items ?? []}
      selectedIds={$documents.selectedIds}
      loading={$documents.loading}
      on:open={(event) => void documents.open(event.detail)}
      on:toggleSelection={(event) => onToggleSelection(event.detail)}
      on:toggleSelectAll={onToggleSelectAll}
      on:bulkAdvanceStatus={() => void documents.bulkAdvanceStatus()}
      on:bulkDelete={() => void documents.bulkDelete()}
    />
  {/if}
</section>

<svelte:window on:keydown={onWindowKeydown} on:click={onWindowClick} />

{#if $documents.editor}
  <DocumentEditorDrawer
    editor={$documents.editor}
    chain={$documents.chain}
    pendingNew={$documents.pendingNew}
    loading={$documents.loading}
    companyName={$shell.state?.chrome.companyName ?? ""}
    counterparties={$counterparties.screen?.items ?? []}
    pendingDirtyClose={pendingDirtyClose}
    pendingDelete={pendingDeleteKind === 'single'}
    bind:chainMenuOpen
    bind:isReassigning
    bind:reassignTargetId
    bind:sectionElement={drawerSection}
    on:close={requestCloseDrawer}
    on:cancelDiscardChanges={cancelDiscardChanges}
    on:confirmDiscardChanges={confirmDiscardChanges}
    on:cancelDelete={cancelDelete}
    on:confirmDelete={confirmDelete}
    on:save={() => documents.save()}
    on:toggleChainMenu={toggleChainMenu}
    on:advanceStatus={onChainMenuAdvanceStatus}
    on:createChainDraft={onDrawerCreateChainDraft}
    on:generatePdf={() => documents.generatePdf()}
    on:createAdjustmentAct={() => { if ($documents.editor) void documents.createAdjustmentActDraft($documents.editor.form.id); }}
    on:deleteCurrent={onDeleteCurrent}
    on:updateFormField={onDrawerUpdateFormField}
    on:openCpCreate={() => void documents.openCpCreate()}
    on:selectCounterparty={onDrawerSelectCounterparty}
    on:openCpEdit={onDrawerOpenCpEdit}
    on:changeCounterparty={onDrawerChangeCounterparty}
  >

    <DocumentItemsEditor
      items={$documents.editor.items}
      loading={$documents.loading}
      on:addItem={() => documents.addItem()}
      on:removeItem={(event) => documents.removeItem(event.detail)}
      on:updateItemField={(event) => documents.updateItemField(event.detail.index, event.detail.field, event.detail.value)}
    />

    {#if supportsExistingPdfFlow($documents.editor.form.kind)}
      <DocumentPdfTools
        documentId={$documents.editor.form.id}
        pdf={$documents.editor.pdf}
        loading={$documents.loading}
        on:attachExistingPdf={onAttachExistingPdf}
        on:openCurrentPdf={onOpenCurrentPdf}
        on:applyTextReplace={(event) => documents.applyPdfTextReplace(event.detail.findText, event.detail.replaceText)}
      />
    {/if}

    {#if $documents.cpModal?.isOpen}
      <CounterpartyModal
        isOpen={$documents.cpModal.isOpen}
        mode={$documents.cpModal.mode}
        form={$documents.cpModal.form}
        loading={$documents.cpModal.loading}
        isDirty={isDirtyCpModal}
        showCloseConfirm={$documents.cpModal.confirmClose}
        on:fieldChange={(e) => documents.updateCpField(e.detail.field, e.detail.value)}
        on:save={async () => { await documents.saveCp(); await counterparties.load(); }}
        on:close={() => documents.closeCpModal()}
        on:closeConfirmed={() => documents.confirmCloseCpModal()}
        on:closeCancelled={() => documents.cancelCloseCpModal()}
      />
    {/if}
  </DocumentEditorDrawer>
{/if}

<style>
  .documents-kind-chips {
    display: flex;
    gap: 6px;
    padding: 8px 16px;
  }

  .kind-chip {
    padding: 4px 12px;
    border-radius: var(--acta-radius-pill);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-subtle);
    cursor: pointer;
    font-size: 13px;
    color: var(--acta-color-text-muted);
  }

  .kind-chip-active {
    background: var(--acta-color-accent);
    border-color: var(--acta-color-accent);
    color: #fff;
  }

</style>
