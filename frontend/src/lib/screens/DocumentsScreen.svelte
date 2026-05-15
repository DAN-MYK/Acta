<script lang="ts">
  import { tick } from "svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import {
    EDITOR_DIRTY_COPY,
    DOCUMENTS_COPY,
    DOCUMENT_DIRECTION_LABELS,
    DOCUMENT_DIRECTION_OPTIONS,
    DOCUMENT_KIND_FILTER_OPTIONS,
    DOCUMENT_KIND_META,
    DOCUMENT_KIND_OPTIONS,
    DOCUMENT_TAB_OPTIONS,
    DOCUMENT_STATUS_OPTIONS,
    DOCUMENTS_FILTER_COPY,
    formatDocumentItemsLabel,
    getDocumentChainTargets,
    getDocumentCreateLabel,
    resolveDocumentKindMeta,
    supportsDocumentPdfGeneration,
    supportsExistingPdfFlow
  } from "../config/ui";
  import { documentsStore } from "../stores/documents";
  import { counterpartiesStore } from "../stores/counterparties";
  import { shellStore } from "../stores/shell";
  import { formatDocumentDraftTotal, formatDocumentItemTotal } from "../documentMoney";
  import { isFormattedMoneyNegative, parseMoneyToMinor } from "../money";
  import type { DocumentDraftItemDto, DocumentKind } from "../types";

  const documents = documentsStore;
  const counterparties = counterpartiesStore;
  const shell = shellStore;

  let createCounterpartyId = "";
  let filterCounterpartyId = "";
  let lastCounterpartyFilterId = "";
  let lastDraftContextCounterpartyId = "";
  let filtersOpen = false;
  let filterButton: HTMLButtonElement | null = null;
  let filterPopover: HTMLElement | null = null;
  let createMenuOpen = false;
  let createMenuButton: HTMLButtonElement | null = null;
  let createMenuPopover: HTMLElement | null = null;
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

  type SortField = "number" | "counterparty" | "date" | "amount" | "kind" | "status" | "direction";
  let sortField: SortField | null = null;
  let sortDir: "asc" | "desc" = "asc";

  function toggleSort(field: SortField) {
    if (sortField === field) {
      if (sortDir === "asc") {
        sortDir = "desc";
      } else {
        sortField = null;
        sortDir = "asc";
      }
    } else {
      sortField = field;
      sortDir = "asc";
    }
  }

  $: sortedItems = (() => {
    const items = $documents.list?.items ?? [];
    if (!sortField) return items;
    const sf = sortField;
    const dir = sortDir === "asc" ? 1 : -1;
    return [...items].sort((a, b) => {
      switch (sf) {
        case "number":      return dir * a.number.localeCompare(b.number, "uk", { numeric: true });
        case "counterparty": return dir * a.counterparty.localeCompare(b.counterparty, "uk");
        case "date":        return dir * a.date.localeCompare(b.date);
        case "amount": {
          const av = parseMoneyToMinor(a.amountStr) ?? 0n;
          const bv = parseMoneyToMinor(b.amountStr) ?? 0n;
          return dir * (av < bv ? -1 : av > bv ? 1 : 0);
        }
        case "kind":      return dir * a.kind.localeCompare(b.kind);
        case "status":    return dir * a.status.localeCompare(b.status);
        case "direction": return dir * a.direction.localeCompare(b.direction);
        default:          return 0;
      }
    });
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
    const nextCounterpartyFilterId = $documents.counterpartyFilterId ?? "";
    if (nextCounterpartyFilterId !== lastCounterpartyFilterId) {
      filterCounterpartyId = nextCounterpartyFilterId;
      lastCounterpartyFilterId = nextCounterpartyFilterId;
    }
  }

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

  function onWindowClick(event: MouseEvent) {
    const target = event.target instanceof Node ? event.target : null;

    if (chainMenuOpen) {
      if (target && chainMenuButton?.contains(target)) return;
      if (target && chainMenuPopover?.contains(target)) return;
      closeChainMenu();
    }

    if (filtersOpen) {
      if (target && filterButton?.contains(target)) return;
      if (target && filterPopover?.contains(target)) return;
      filtersOpen = false;
    }

    if (createMenuOpen) {
      if (target && createMenuButton?.contains(target)) return;
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

  const documentKindMeta = DOCUMENT_KIND_META;

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

  // Panel draft state
  let panelDateFrom: string = "";
  let panelDateTo: string = "";
  let panelStatuses: string[] = [];
  let panelAmountMin: string = "";
  let panelAmountMax: string = "";

  $: if (filtersOpen) {
    // sync draft from current store state when panel opens
    panelDateFrom = $documents.dateFrom ?? "";
    panelDateTo = $documents.dateTo ?? "";
    panelStatuses = [...$documents.statusFilter];
    panelAmountMin = $documents.amountMin ?? "";
    panelAmountMax = $documents.amountMax ?? "";
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

  $: dateRangeError = (panelDateFrom && panelDateTo && panelDateFrom > panelDateTo)
    ? DOCUMENTS_FILTER_COPY.errors.dateRangeInvalid
    : null;

  $: amountRangeError = computeAmountError(panelAmountMin, panelAmountMax);

  function computeAmountError(minStr: string, maxStr: string): string | null {
    const norm = (s: string) => s.trim().replace(/\s+/g, "").replace(",", ".");
    const minNum = minStr ? Number(norm(minStr)) : null;
    const maxNum = maxStr ? Number(norm(maxStr)) : null;
    if (minStr && (minNum === null || Number.isNaN(minNum))) return DOCUMENTS_FILTER_COPY.errors.amountInvalidFormat;
    if (maxStr && (maxNum === null || Number.isNaN(maxNum))) return DOCUMENTS_FILTER_COPY.errors.amountInvalidFormat;
    if (minNum !== null && maxNum !== null && minNum > maxNum) return DOCUMENTS_FILTER_COPY.errors.amountRangeInvalid;
    return null;
  }

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

  function toggleStatus(code: string, on: boolean) {
    panelStatuses = on
      ? Array.from(new Set([...panelStatuses, code]))
      : panelStatuses.filter((s) => s !== code);
  }

  function onDateSubpreset(kind: 'today' | 'week' | 'month' | 'quarter' | 'year') {
    const today = new Date();
    const iso = (d: Date) => d.toISOString().slice(0, 10);
    if (kind === 'today') { panelDateFrom = iso(today); panelDateTo = iso(today); return; }
    if (kind === 'week') {
      const start = new Date(today); start.setDate(today.getDate() - 6);
      panelDateFrom = iso(start); panelDateTo = iso(today); return;
    }
    if (kind === 'month') {
      panelDateFrom = iso(new Date(today.getFullYear(), today.getMonth(), 1));
      panelDateTo = iso(today); return;
    }
    if (kind === 'quarter') {
      const q = Math.floor(today.getMonth() / 3);
      panelDateFrom = iso(new Date(today.getFullYear(), q * 3, 1));
      panelDateTo = iso(today); return;
    }
    if (kind === 'year') {
      panelDateFrom = iso(new Date(today.getFullYear(), 0, 1));
      panelDateTo = iso(today); return;
    }
  }

  function resetPanelDraft() {
    panelDateFrom = "";
    panelDateTo = "";
    panelStatuses = [];
    panelAmountMin = "";
    panelAmountMax = "";
    filterCounterpartyId = "";
  }

  function normalizeAmount(s: string): string | null {
    const n = s.trim().replace(/\s+/g, "").replace(",", ".");
    return n.length === 0 ? null : n;
  }

  function applyPanel() {
    if (dateRangeError || amountRangeError) return;
    void documents.applyFilters({
      dateFrom: panelDateFrom || null,
      dateTo: panelDateTo || null,
      statusFilter: [...panelStatuses],
      amountMin: normalizeAmount(panelAmountMin),
      amountMax: normalizeAmount(panelAmountMax),
      counterpartyFilterId: filterCounterpartyId || null,
    });
    filtersOpen = false;
  }

  $: selectedCreateKind = $documents.kindFilter;

  $: createButtonKind = selectedCreateKind ?? null;

  $: createButtonLabel = createButtonKind
    ? getDocumentCreateLabel(createButtonKind, $documents.activeTab)
    : "Створити ▾";

  function onCreateDraft() {
    if (!createButtonKind) {
      createMenuOpen = !createMenuOpen;
      return;
    }
    if (createCounterpartyId) {
      void documents.create(createCounterpartyId, createButtonKind);
    } else {
      documents.openNewEditor(createButtonKind);
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

  function onEditorCounterpartyChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    const id = select.value;
    const name = select.options[select.selectedIndex]?.text ?? "";
    documents.updateCounterparty(id, id ? name : "");
  }

  function toggleFilters() {
    filtersOpen = !filtersOpen;
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

  let pendingDeleteKind: 'single' | 'bulk' | null = null;

  function onDeleteCurrent() {
    pendingDeleteKind = 'single';
  }

  function confirmDelete() {
    if (pendingDeleteKind === 'single') {
      void documents.deleteCurrent();
    } else if (pendingDeleteKind === 'bulk') {
      void documents.bulkDelete();
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
    pendingDeleteKind = 'bulk';
  }

  function onBulkAdvanceStatus() {
    void documents.bulkAdvanceStatus();
  }

  function getDocumentKindLabel(kind: string): string {
    return resolveDocumentKindMeta(kind).label;
  }

  const navTabs = DOCUMENT_TAB_OPTIONS;

  const kindChips = DOCUMENT_KIND_FILTER_OPTIONS;

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
          <div
            bind:this={filterPopover}
            id="documents-filter-popover"
            class="filter-popover"
            data-testid="documents-filter-panel"
            role="dialog"
            aria-label="Фільтр документів"
          >
            <fieldset class="filter-panel-section">
              <legend>{DOCUMENTS_FILTER_COPY.periodLabel}</legend>
              <div class="filter-panel-subpresets">
                <button type="button" class="kind-chip" on:click={() => onDateSubpreset('today')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.today}</button>
                <button type="button" class="kind-chip" on:click={() => onDateSubpreset('week')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.week}</button>
                <button type="button" class="kind-chip" on:click={() => onDateSubpreset('month')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.month}</button>
                <button type="button" class="kind-chip" on:click={() => onDateSubpreset('quarter')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.quarter}</button>
                <button type="button" class="kind-chip" on:click={() => onDateSubpreset('year')}>{DOCUMENTS_FILTER_COPY.periodSubpresets.year}</button>
              </div>
              <div class="filter-panel-grid-2">
                <label><span>{DOCUMENTS_FILTER_COPY.periodFrom}</span><input type="date" bind:value={panelDateFrom} /></label>
                <label><span>{DOCUMENTS_FILTER_COPY.periodTo}</span><input type="date" bind:value={panelDateTo} /></label>
              </div>
              {#if dateRangeError}
                <p class="filter-error" role="alert">{dateRangeError}</p>
              {/if}
            </fieldset>

            <fieldset class="filter-panel-section">
              <legend>{DOCUMENTS_FILTER_COPY.statusLabel}</legend>
              <div class="filter-panel-statuses">
                {#each DOCUMENT_STATUS_OPTIONS as opt}
                  <label class="status-checkbox">
                    <input type="checkbox" value={opt.value}
                      checked={panelStatuses.includes(opt.value)}
                      on:change={() => toggleStatus(opt.value, !panelStatuses.includes(opt.value))} />
                    {opt.label}
                  </label>
                {/each}
              </div>
            </fieldset>

            <fieldset class="filter-panel-section">
              <legend>{DOCUMENTS_FILTER_COPY.counterpartyLabel}</legend>
              <select
                bind:value={filterCounterpartyId}
                disabled={$documents.loading}
                data-testid="documents-counterparty-filter"
                aria-label="Фільтр за контрагентом"
              >
                <option value="">{DOCUMENTS_FILTER_COPY.counterpartyAll}</option>
                {#each $counterparties.screen?.items ?? [] as cp}
                  <option value={cp.id}>{cp.name}</option>
                {/each}
              </select>
            </fieldset>

            <fieldset class="filter-panel-section">
              <legend>{DOCUMENTS_FILTER_COPY.amountLabel}</legend>
              <div class="filter-panel-grid-2">
                <label><span>{DOCUMENTS_FILTER_COPY.amountFrom}</span><input type="text" inputmode="decimal" bind:value={panelAmountMin} placeholder="0,00" /></label>
                <label><span>{DOCUMENTS_FILTER_COPY.amountTo}</span><input type="text" inputmode="decimal" bind:value={panelAmountMax} placeholder="0,00" /></label>
              </div>
              {#if amountRangeError}
                <p class="filter-error" role="alert">{amountRangeError}</p>
              {/if}
            </fieldset>

            <div class="documents-filter-actions">
              <button class="btn-ghost" type="button" on:click={resetPanelDraft} disabled={$documents.loading}>
                {DOCUMENTS_FILTER_COPY.reset}
              </button>
              <button class="btn-primary" type="button" on:click={applyPanel} disabled={$documents.loading || !!dateRangeError || !!amountRangeError}>
                {DOCUMENTS_FILTER_COPY.apply}
              </button>
            </div>
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

      <div class="documents-toolbar-popover-anchor">
        <button
          bind:this={createMenuButton}
          class="btn-primary"
          data-testid="documents-create-button"
          type="button"
          disabled={$documents.loading}
          on:click={onCreateDraft}
          aria-expanded={createMenuOpen}
          aria-controls={createMenuOpen ? "documents-create-picker" : undefined}
          aria-busy={$documents.loading ? "true" : "false"}
        >
          {#if createButtonKind}
            <AppIcon name={resolveDocumentKindMeta(createButtonKind).icon} surface={true} />
          {/if}
          <span>{createButtonLabel}</span>
        </button>

        {#if createMenuOpen}
          <div
            bind:this={createMenuPopover}
            id="documents-create-picker"
            class="create-picker-popover"
            data-testid="documents-create-picker"
            role="menu"
            aria-label="Створити документ"
          >
            {#each DOCUMENT_KIND_OPTIONS as option}
              <button
                type="button"
                class="create-picker-item"
                data-testid={`documents-create-picker-${option.value}`}
                role="menuitem"
                on:click={() => onCreateMenuKind(option.value)}
              >
                <AppIcon name={documentKindMeta[option.value].icon} surface={true} />
                <span>{option.label}</span>
              </button>
            {/each}
          </div>
        {/if}
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

  {#if ($documents.list?.items.length ?? 0) > 0}
  <div
    class="bulk-actions"
    class:bulk-actions-idle={$documents.selectedIds.length === 0}
    data-testid="documents-bulk-actions"
  >
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
  {/if}

  {#if pendingDeleteKind === 'bulk'}
    <div
      class="confirm-delete-banner"
      role="alertdialog"
      aria-live="assertive"
      aria-labelledby="documents-confirm-bulk-title"
      data-testid="documents-confirm-bulk-banner"
    >
      <div>
        <strong id="documents-confirm-bulk-title">Видалити вибрані?</strong>
        <p>{DOCUMENTS_COPY.confirmDeleteBulk}</p>
      </div>
      <div class="editor-dirty-actions">
        <button type="button" class="btn-ghost btn-sm" on:click={cancelDelete}>Скасувати</button>
        <button type="button" class="btn-danger btn-sm" on:click={confirmDelete} data-testid="documents-confirm-bulk-confirm">Видалити</button>
      </div>
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
    <div class="documents-table-card" data-testid="documents-list">
      <div class="documents-table-scroll">
        <div class="doc-trow doc-trow-head">
          <div></div>
          <button class="doc-sort-btn" type="button" on:click={() => toggleSort("number")} aria-label="Сортувати за номером" data-active={sortField === "number" || null}>
            <span>Номер</span>
            <svg width="10" height="12" viewBox="0 0 10 12" aria-hidden="true" fill="currentColor">
              <path d="M5 1.5L2 5h6L5 1.5Z" opacity={sortField === "number" ? (sortDir === "asc" ? 1 : 0.22) : 0.38}/>
              <path d="M5 10.5L8 7H2l3 3.5Z" opacity={sortField === "number" ? (sortDir === "desc" ? 1 : 0.22) : 0.38}/>
            </svg>
          </button>
          <button class="doc-sort-btn" type="button" on:click={() => toggleSort("counterparty")} aria-label="Сортувати за контрагентом" data-active={sortField === "counterparty" || null}>
            <span>Контрагент</span>
            <svg width="10" height="12" viewBox="0 0 10 12" aria-hidden="true" fill="currentColor">
              <path d="M5 1.5L2 5h6L5 1.5Z" opacity={sortField === "counterparty" ? (sortDir === "asc" ? 1 : 0.22) : 0.38}/>
              <path d="M5 10.5L8 7H2l3 3.5Z" opacity={sortField === "counterparty" ? (sortDir === "desc" ? 1 : 0.22) : 0.38}/>
            </svg>
          </button>
          <button class="doc-sort-btn" type="button" on:click={() => toggleSort("date")} aria-label="Сортувати за датою" data-active={sortField === "date" || null}>
            <span>Дата</span>
            <svg width="10" height="12" viewBox="0 0 10 12" aria-hidden="true" fill="currentColor">
              <path d="M5 1.5L2 5h6L5 1.5Z" opacity={sortField === "date" ? (sortDir === "asc" ? 1 : 0.22) : 0.38}/>
              <path d="M5 10.5L8 7H2l3 3.5Z" opacity={sortField === "date" ? (sortDir === "desc" ? 1 : 0.22) : 0.38}/>
            </svg>
          </button>
          <button class="doc-sort-btn doc-sort-btn-right" type="button" on:click={() => toggleSort("amount")} aria-label="Сортувати за сумою" data-active={sortField === "amount" || null}>
            <span>Сума</span>
            <svg width="10" height="12" viewBox="0 0 10 12" aria-hidden="true" fill="currentColor">
              <path d="M5 1.5L2 5h6L5 1.5Z" opacity={sortField === "amount" ? (sortDir === "asc" ? 1 : 0.22) : 0.38}/>
              <path d="M5 10.5L8 7H2l3 3.5Z" opacity={sortField === "amount" ? (sortDir === "desc" ? 1 : 0.22) : 0.38}/>
            </svg>
          </button>
          <button class="doc-sort-btn" type="button" on:click={() => toggleSort("kind")} aria-label="Сортувати за типом" data-active={sortField === "kind" || null}>
            <span>Тип</span>
            <svg width="10" height="12" viewBox="0 0 10 12" aria-hidden="true" fill="currentColor">
              <path d="M5 1.5L2 5h6L5 1.5Z" opacity={sortField === "kind" ? (sortDir === "asc" ? 1 : 0.22) : 0.38}/>
              <path d="M5 10.5L8 7H2l3 3.5Z" opacity={sortField === "kind" ? (sortDir === "desc" ? 1 : 0.22) : 0.38}/>
            </svg>
          </button>
          <button class="doc-sort-btn" type="button" on:click={() => toggleSort("status")} aria-label="Сортувати за статусом" data-active={sortField === "status" || null}>
            <span>Статус</span>
            <svg width="10" height="12" viewBox="0 0 10 12" aria-hidden="true" fill="currentColor">
              <path d="M5 1.5L2 5h6L5 1.5Z" opacity={sortField === "status" ? (sortDir === "asc" ? 1 : 0.22) : 0.38}/>
              <path d="M5 10.5L8 7H2l3 3.5Z" opacity={sortField === "status" ? (sortDir === "desc" ? 1 : 0.22) : 0.38}/>
            </svg>
          </button>
          <button class="doc-sort-btn" type="button" on:click={() => toggleSort("direction")} aria-label="Сортувати за напрямком" data-active={sortField === "direction" || null}>
            <span>Напрямок</span>
            <svg width="10" height="12" viewBox="0 0 10 12" aria-hidden="true" fill="currentColor">
              <path d="M5 1.5L2 5h6L5 1.5Z" opacity={sortField === "direction" ? (sortDir === "asc" ? 1 : 0.22) : 0.38}/>
              <path d="M5 10.5L8 7H2l3 3.5Z" opacity={sortField === "direction" ? (sortDir === "desc" ? 1 : 0.22) : 0.38}/>
            </svg>
          </button>
        </div>

        {#each sortedItems as item}
          <div class="doc-trow doc-trow-data" data-testid={`documents-row-${item.id}`}>
            <button
              class="doc-row-open"
              type="button"
              on:click={() => documents.open(item.id)}
              disabled={$documents.loading}
              aria-label={`Відкрити документ ${item.number}`}
            ></button>

            <label class="doc-row-checkbox doc-tcell" aria-label={`Вибрати ${item.number}`}>
              <input
                type="checkbox"
                checked={$documents.selectedIds.includes(item.id)}
                on:click|stopPropagation={() => onToggleSelection(item.id)}
              />
            </label>

            <span class="doc-tcell doc-tcell-number">
              <AppIcon name={resolveDocumentKindMeta(item.kind).icon} surface={true} size={16} />
              <span>{item.number}</span>
            </span>
            <span class="doc-tcell doc-tcell-counterparty">{item.counterparty}</span>
            <span class="doc-tcell doc-tcell-date">{item.date}</span>
            <span class="doc-tcell doc-tcell-amount money-value" data-negative={isFormattedMoneyNegative(item.amountStr)}>{item.amountStr}</span>
            <span class="doc-tcell doc-tcell-kind">
              <span class="doc-kind-badge">
                <AppIcon name={resolveDocumentKindMeta(item.kind).icon} size={14} />
                <span>{getDocumentKindLabel(item.kind)}</span>
              </span>
            </span>
            <span class="doc-tcell doc-tcell-status">
              <span class="doc-status-chip">{item.statusLabel}</span>
            </span>
            <span class="doc-tcell doc-tcell-direction">
              <span class="doc-direction-badge" data-direction={item.direction}>
                {DOCUMENT_DIRECTION_LABELS[item.direction] ?? item.direction}
              </span>
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</section>

<svelte:window on:keydown={onWindowKeydown} on:click={onWindowClick} />

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
          <strong id="documents-dirty-banner-title">{EDITOR_DIRTY_COPY.dirtyTitle}</strong>
          <p>{EDITOR_DIRTY_COPY.dirtyDescription}</p>
        </div>
        <div class="editor-dirty-actions">
          <button
            type="button"
            class="btn-ghost btn-sm"
            on:click={cancelDiscardChanges}
            data-testid="documents-dirty-banner-cancel"
          >
            {EDITOR_DIRTY_COPY.dirtyStay}
          </button>
          <button
            type="button"
            class="btn-danger btn-sm"
            on:click={confirmDiscardChanges}
            data-testid="documents-dirty-banner-discard"
          >
            {EDITOR_DIRTY_COPY.dirtyDiscard}
          </button>
        </div>
      </div>
    {/if}

    {#if pendingDeleteKind === 'single'}
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
          <button type="button" class="btn-ghost btn-sm" on:click={cancelDelete}>Скасувати</button>
          <button type="button" class="btn-danger btn-sm" on:click={confirmDelete} data-testid="documents-confirm-delete-confirm">Видалити</button>
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
          {#if chainMenuOpen}
            <div
              bind:this={chainMenuPopover}
              class="chain-menu-popover"
              role="menu"
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
              {#each getDocumentChainTargets($documents.editor.form.kind) as targetKind}
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
          {/if}
        </div>

        {#if supportsDocumentPdfGeneration($documents.editor.form.kind)}
          <button class="btn-ghost" on:click={() => documents.generatePdf()} disabled={$documents.loading}>
            PDF
          </button>
        {/if}
        <div class="editor-actions-close">
          <button class="btn-danger" on:click={onDeleteCurrent} disabled={$documents.loading} data-testid="documents-delete-current-btn">
            Видалити
          </button>
          <button class="btn-ghost" on:click={requestCloseDrawer} disabled={$documents.loading}>
            Закрити
          </button>
        </div>
      </div>
    </div>

    <div class="editor-grid">
      <div class="editor-field-readonly editor-grid-span">
        <span class="editor-field-readonly-label">Компанія</span>
        <span class="editor-field-readonly-value">{$shell.state?.chrome.companyName ?? ""}</span>
        <span class="editor-field-readonly-hint">тільки перегляд</span>
      </div>

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
          {DOCUMENT_DIRECTION_OPTIONS[0].label}
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
          {DOCUMENT_DIRECTION_OPTIONS[1].label}
        </label>
      </fieldset>

      <label>
        Номер
        <input value={$documents.editor.form.number} on:input={onEditorNumberChange} disabled={$documents.loading} placeholder="Буде згенеровано автоматично" />
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

      {#if $documents.pendingNew}
        <label class="editor-grid-span">
          Контрагент
          <select
            value={$documents.editor.form.counterpartyId}
            on:change={onEditorCounterpartyChange}
            disabled={$documents.loading}
            required
          >
            <option value="">— Оберіть контрагента —</option>
            {#each $counterparties.screen?.items ?? [] as cp}
              <option value={cp.id}>{cp.name}</option>
            {/each}
          </select>
        </label>
      {:else}
        <div class="editor-field-readonly editor-grid-span">
          <span class="editor-field-readonly-label">Контрагент</span>
          <span class="editor-field-readonly-value">{$documents.editor.form.counterpartyName}</span>
          <span class="editor-field-readonly-hint">тільки перегляд</span>
        </div>
      {/if}
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

    <label class="editor-notes-field">
      Примітки
      <textarea
        rows="3"
        value={$documents.editor.form.notes}
        on:input={onEditorNotesChange}
        disabled={$documents.loading}
      ></textarea>
    </label>

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

  .doc-direction-badge {
    font-size: 11px;
    color: var(--acta-color-text-faint);
  }

  .doc-direction-badge[data-direction="outgoing"] {
    color: var(--acta-color-success);
  }

  .doc-direction-badge[data-direction="incoming"] {
    color: var(--acta-color-warning);
  }

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
</style>
