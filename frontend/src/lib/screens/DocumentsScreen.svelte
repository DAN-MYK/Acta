<script lang="ts">
  import AppIcon from "../components/AppIcon.svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import type { AppIconName } from "../icons";
  import { documentsStore } from "../stores/documents";
  import { counterpartiesStore } from "../stores/counterparties";
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

  const documentKindLabels: Record<DocumentKind, string> = {
    invoice: "Рахунок",
    act: "Акт",
    waybill: "Накладна"
  };

  const documentKindIcons: Record<DocumentKind, AppIconName> = {
    invoice: "invoice",
    act: "act",
    waybill: "waybill"
  };

  const documentKindActionLabels: Record<DocumentKind, string> = {
    invoice: "рахунок",
    act: "акт",
    waybill: "накладну"
  };

  const itemTotalFormatter = new Intl.NumberFormat("uk-UA", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });

  interface DecimalValue {
    value: bigint;
    scale: number;
  }

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
    if (!window.confirm("Видалити поточний документ? Цю дію не можна скасувати.")) {
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
    if (!window.confirm("Видалити вибрані документи? Цю дію не можна скасувати.")) {
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

  function getDocumentKindIcon(kind: string): AppIconName {
    const normalized = kind.toLowerCase();

    if (normalized === "invoice" || normalized.includes("рах")) {
      return "invoice";
    }
    if (normalized === "act" || normalized.includes("акт")) {
      return "act";
    }
    if (normalized === "waybill" || normalized.includes("наклад")) {
      return "waybill";
    }
    if (normalized.includes("догов")) {
      return "contract";
    }
    if (normalized.includes("pdf")) {
      return "pdf";
    }
    if (normalized.includes("excel") || normalized.includes("xls")) {
      return "excel";
    }
    return "documents";
  }

  function getDocumentKindLabel(kind: string): string {
    const normalized = kind.toLowerCase();

    if (normalized === "invoice" || normalized.includes("рах")) {
      return "Рахунок";
    }
    if (normalized === "act" || normalized.includes("акт")) {
      return "Акт";
    }
    if (normalized === "waybill" || normalized.includes("наклад")) {
      return "Накладна";
    }
    if (normalized.includes("догов")) {
      return "Договір";
    }
    if (normalized.includes("pdf")) {
      return "PDF";
    }
    if (normalized.includes("excel") || normalized.includes("xls")) {
      return "Excel";
    }
    return kind;
  }

  function getCreateButtonLabel(kind: DocumentKind): string {
    if (kind === "invoice") {
      return "Створити рахунок";
    }
    if (kind === "waybill") {
      return "Створити накладну";
    }
    return "Створити акт";
  }

  function getItemsCountLabel(count: number): string {
    if (count === 1) {
      return "1 позиція";
    }
    if (count >= 2 && count <= 4) {
      return `${count} позиції`;
    }
    return `${count} позицій`;
  }

  function pow10(exponent: number): bigint {
    let result = 1n;

    for (let index = 0; index < exponent; index += 1) {
      result *= 10n;
    }

    return result;
  }

  function parseDecimal(value: string): DecimalValue | null {
    const normalized = value.replace(/\s+/g, "").replace(",", ".").trim();
    if (!normalized) {
      return null;
    }

    const match = normalized.match(/^(-?)(\d+)(?:\.(\d+))?$/);
    if (!match) {
      return null;
    }

    const [, sign, integerPart, fractionalPart = ""] = match;
    const digits = `${integerPart}${fractionalPart}`.replace(/^0+(?=\d)/, "") || "0";

    return {
      value: sign === "-" ? -BigInt(digits) : BigInt(digits),
      scale: fractionalPart.length
    };
  }

  function multiplyDecimals(left: string, right: string): DecimalValue | null {
    const leftDecimal = parseDecimal(left);
    const rightDecimal = parseDecimal(right);

    if (!leftDecimal || !rightDecimal) {
      return null;
    }

    return {
      value: leftDecimal.value * rightDecimal.value,
      scale: leftDecimal.scale + rightDecimal.scale
    };
  }

  function addDecimalValues(current: DecimalValue, next: DecimalValue): DecimalValue {
    if (current.scale === next.scale) {
      return {
        value: current.value + next.value,
        scale: current.scale
      };
    }

    if (current.scale > next.scale) {
      return {
        value: current.value + next.value * pow10(current.scale - next.scale),
        scale: current.scale
      };
    }

    return {
      value: current.value * pow10(next.scale - current.scale) + next.value,
      scale: next.scale
    };
  }

  function formatScaledMoney(decimal: DecimalValue): string {
    const negative = decimal.value < 0n;
    const absoluteValue = negative ? -decimal.value : decimal.value;
    let roundedMinorUnits: bigint;

    if (decimal.scale > 2) {
      const divisor = pow10(decimal.scale - 2);
      roundedMinorUnits = (absoluteValue + divisor / 2n) / divisor;
    } else if (decimal.scale < 2) {
      roundedMinorUnits = absoluteValue * pow10(2 - decimal.scale);
    } else {
      roundedMinorUnits = absoluteValue;
    }

    const integerPart = roundedMinorUnits / 100n;
    const fractionalPart = (roundedMinorUnits % 100n).toString().padStart(2, "0");
    const groupedIntegerPart = integerPart.toString().replace(/\B(?=(\d{3})+(?!\d))/g, " ");

    return `${negative ? "-" : ""}${groupedIntegerPart},${fractionalPart} грн`;
  }

  function formatItemTotal(quantity: string, price: string): string {
    const total = multiplyDecimals(quantity, price);
    return total ? formatScaledMoney(total) : "—";
  }

  function totalDraftAmount(items: DocumentDraftItemDto[]): string {
    let total: DecimalValue = { value: 0n, scale: 0 };

    for (const item of items) {
      const itemTotal = multiplyDecimals(item.quantity, item.price);
      if (!itemTotal) {
        continue;
      }

      total = addDecimalValues(total, itemTotal);
    }

    return formatScaledMoney(total);
  }

  function getCurrentChainStatus() {
    const steps = $documents.chain?.steps ?? [];
    return steps.length > 0 ? steps[steps.length - 1].status : "Чернетка";
  }

  function getEditorKindIcon(kind: string): AppIconName {
    return getDocumentKindIcon(kind);
  }

  function supportsExistingPdfFlow(kind: string): boolean {
    return kind === "invoice" || kind === "waybill";
  }
</script>

<section class="panel" data-testid="documents-screen">
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
      <option value="act">Акт</option>
      <option value="invoice">Рахунок</option>
      <option value="waybill">Накладна</option>
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
      <AppIcon name={documentKindIcons[createKind]} surface={true} />
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
      <strong>Поки що документів немає</strong>
      <p>Почніть зі створення першого рахунку, акта або накладної, щоб запустити повний сценарій документа.</p>
      <div class="empty-state-actions">
        <button
          class="btn-primary"
          type="button"
          data-testid="documents-empty-primary-action"
          on:click={focusCreateButton}
        >
          РЎС‚РІРѕСЂРёС‚Рё РїРµСЂС€РёР№ РґРѕРєСѓРјРµРЅС‚
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
                  <AppIcon name={getDocumentKindIcon(item.kind)} surface={true} size={16} />
                  <span>{item.number}</span>
                </strong>
                <p>{item.counterparty}</p>
              </div>
              <div class="doc-row-meta">
                <span>{item.date}</span>
                <span class="money-value" data-negative={item.amountStr.trim().startsWith("-")}>{item.amountStr}</span>
                <span class="doc-kind-badge">
                  <AppIcon name={getDocumentKindIcon(item.kind)} size={14} />
                  <span>{getDocumentKindLabel(item.kind)}</span>
                </span>
                <span class="doc-status-chip">{item.statusLabel}</span>
              </div>
            </div>
          </button>
        </div>
      {/each}
    </div>
  {/if}
</section>

{#if $documents.editor}
  <section class="editor-sheet">
    <div class="editor-header">
      <div>
        <div class="editor-header-meta">
          <span class="doc-kind-badge">
            <AppIcon name={getEditorKindIcon($documents.editor.form.kind)} size={14} />
            <span>{getDocumentKindLabel($documents.editor.form.kind)}</span>
          </span>
          <span class="doc-status-chip">{getCurrentChainStatus()}</span>
        </div>
        <h3>{$documents.editor.form.title}</h3>
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
        <button class="btn-secondary" on:click={() => documents.advanceStatus()} disabled={$documents.loading}>
          Наступний статус
        </button>
        {#each getChainTargets($documents.editor.form.kind) as targetKind}
          <button
            class="btn-secondary"
            data-testid={`documents-chain-create-${targetKind}`}
            on:click={() => onCreateChainDraft(targetKind)}
            disabled={$documents.loading}
          >
            <AppIcon name={documentKindIcons[targetKind]} size={16} />
            <span>Створити {documentKindActionLabels[targetKind]}</span>
          </button>
        {/each}
        {#if ["act", "invoice"].includes($documents.editor.form.kind)}
          <button class="btn-ghost" on:click={() => documents.generatePdf()} disabled={$documents.loading}>
            PDF
          </button>
        {/if}
        <button class="btn-danger" on:click={onDeleteCurrent} disabled={$documents.loading}>
          Видалити
        </button>
        <button class="btn-ghost" on:click={() => documents.closeEditor()} disabled={$documents.loading}>
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
    </div>

    <div class="editor-items-card">
      <div class="editor-items-header">
        <strong>Позиції документа</strong>
        <div class="editor-items-summary">
          <span class="editor-items-count">{getItemsCountLabel($documents.editor.items.length)}</span>
          <strong>{totalDraftAmount($documents.editor.items)}</strong>
          <button class="btn-secondary" on:click={() => documents.addItem()} disabled={$documents.loading}>
            Додати позицію
          </button>
        </div>
      </div>

      <div class="editor-items">
        {#if $documents.editor.items.length === 0}
          <div class="editor-items-empty" data-testid="documents-items-empty">
            <strong>Поки що без позицій</strong>
            <p>Додайте першу позицію, щоб менеджер одразу бачив номенклатуру, кількість, ціну й підсумок документа.</p>
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
            <div class="editor-item-card">
              <div class="editor-item-meta">
                <strong>Рядок {index + 1}</strong>
                <span>Сума позиції {formatItemTotal(item.quantity, item.price)}</span>
              </div>

              <div class="editor-item">
                <label class="editor-item-field">
                  <span>Опис</span>
                  <input
                    value={item.description}
                    placeholder="Опишіть товар або послугу"
                    on:input={(event) => onItemFieldChange(index, "description", event)}
                    disabled={$documents.loading}
                  />
                </label>
                <label class="editor-item-field">
                  <span>Од.</span>
                  <input
                    value={item.unit}
                    placeholder="шт / год / посл."
                    on:input={(event) => onItemFieldChange(index, "unit", event)}
                    disabled={$documents.loading}
                  />
                </label>
                <label class="editor-item-field editor-item-field-numeric">
                  <span>Кількість</span>
                  <input
                    value={item.quantity}
                    placeholder="0"
                    inputmode="decimal"
                    on:input={(event) => onItemFieldChange(index, "quantity", event)}
                    disabled={$documents.loading}
                  />
                </label>
                <label class="editor-item-field editor-item-field-numeric">
                  <span>Ціна, грн</span>
                  <input
                    value={item.price}
                    placeholder="0,00"
                    inputmode="decimal"
                    on:input={(event) => onItemFieldChange(index, "price", event)}
                    disabled={$documents.loading}
                  />
                </label>
                <div class="editor-item-total" aria-label={`Сума рядка ${index + 1}`}>
                  <span>Сума</span>
                  <strong>{formatItemTotal(item.quantity, item.price)}</strong>
                </div>
                <button
                  class="btn-danger editor-item-remove"
                  on:click={() => documents.removeItem(index)}
                  disabled={$documents.loading}
                >
                  Видалити позицію
                </button>
              </div>
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
            <p>
              Підтриманий сценарій: прив’язати існуючий PDF, прочитати текстовий шар і виконати exact
              replace у підтримуваному content stream. Це не універсальний PDF-редактор.
            </p>
          </div>
          <div class="editor-actions existing-pdf-actions">
            <button class="btn-secondary" on:click={onAttachExistingPdf} disabled={$documents.loading}>
              {$documents.editor.pdf ? "Прив’язати інший PDF" : "Прив’язати PDF"}
            </button>
            <button
              class="btn-ghost"
              on:click={onOpenCurrentPdf}
              disabled={$documents.loading || !$documents.editor.pdf}
            >
              Відкрити PDF
            </button>
          </div>
        </div>

        {#if $documents.editor.pdf}
          <div class="chain-summary">
            <div class="chain-summary-block chain-summary-block-wide">
              <span>Файл</span>
              <strong>{$documents.editor.pdf.filePath}</strong>
            </div>
            <div class="chain-summary-block">
              <span>Сторінки</span>
              <strong>{$documents.editor.pdf.pageCount}</strong>
            </div>
            <div class="chain-summary-block">
              <span>Текстовий шар</span>
              <strong>{$documents.editor.pdf.hasTextOps ? "Знайдено" : "Не знайдено"}</strong>
            </div>
            <div class="chain-summary-block">
              <span>Exact replace</span>
              <strong>{$documents.editor.pdf.editable ? "Підтримується" : "Непідтримується"}</strong>
            </div>
          </div>

          {#if $documents.editor.pdf.warnings.length > 0}
            <div class="existing-pdf-warnings">
              {#each $documents.editor.pdf.warnings as warning}
                <p>{warning}</p>
              {/each}
            </div>
          {/if}

          <label class="editor-grid-span existing-pdf-preview">
            Витягнутий текст
            <textarea rows="10" readonly value={$documents.editor.pdf.extractedText}></textarea>
          </label>

          <div class="editor-item existing-pdf-replace">
            <label class="editor-item-field">
              <span>Знайти текст</span>
              <input bind:value={pdfFindText} placeholder="Точний фрагмент з витягнутого тексту" />
            </label>
            <label class="editor-item-field">
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
        {:else}
          <div class="editor-items-empty" data-testid="documents-existing-pdf-empty">
            <strong>PDF ще не прив’язано</strong>
            <p>
              Прив’яжіть існуючий PDF до цього документа, щоб переглянути текстовий шар і виконати
              підтримувану заміну тексту без перезапису оригіналу.
            </p>
            <button class="btn-primary" on:click={onAttachExistingPdf} disabled={$documents.loading}>
              Прив’язати перший PDF
            </button>
          </div>
        {/if}
      </div>
    {/if}
  </section>
{/if}
