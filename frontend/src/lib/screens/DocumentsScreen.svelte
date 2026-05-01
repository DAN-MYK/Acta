<script lang="ts">
  import AppIcon from "../components/AppIcon.svelte";
  import type { AppIconName } from "../icons";
  import { documentsStore } from "../stores/documents";
  import { counterpartiesStore } from "../stores/counterparties";
  import type { DocumentKind, DocumentItemDto } from "../types";

  const documents = documentsStore;
  const counterparties = counterpartiesStore;

  let createCounterpartyId = "";
  let createKind: DocumentKind = "act";

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
    invoice: "Рахунок",
    act: "Акт",
    waybill: "Накладну"
  };

  const itemTotalFormatter = new Intl.NumberFormat("uk-UA", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });

  $: if ($documents.draftContext?.counterpartyId) {
    createCounterpartyId = $documents.draftContext.counterpartyId;
  }

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

  function onToggleSelection(docId: string) {
    documents.toggleSelected(docId);
  }

  function onToggleSelectAll() {
    documents.selectAllVisible();
  }

  function onBulkDelete() {
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

  function getCreateHint(counterpartyId: string, kind: DocumentKind): string {
    if (!counterpartyId) {
      return "Спочатку оберіть контрагента, щоб ми відкрили чернетку в правильному контексті.";
    }

    return `Чернетка типу "${documentKindLabels[kind]}" відкриється одразу з прив'язкою до вибраного контрагента.`;
  }

  function getNextStepMessage(kind: string): string {
    if (kind === "invoice") {
      return "На основі рахунку можна одразу підготувати акт або накладну.";
    }
    if (kind === "act") {
      return "Після акту зазвичай лишається підготувати накладну або оновити статус документа.";
    }
    if (kind === "waybill") {
      return "Накладна вже закриває сценарій відвантаження, тож далі варто лише перевірити статус і зв'язки.";
    }

    return "Перевірте, який похідний документ потрібен далі, і створіть його звідси.";
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

  function formatItemTotal(quantity: string, price: string): string {
    const normalizedQuantity = Number.parseFloat(quantity.replace(",", "."));
    const normalizedPrice = Number.parseFloat(price.replace(",", "."));

    if (!Number.isFinite(normalizedQuantity) || !Number.isFinite(normalizedPrice)) {
      return "—";
    }

    return `${itemTotalFormatter
      .format(normalizedQuantity * normalizedPrice)
      .replace(/\u00A0/g, " ")} грн`;
  }

  function draftCount(items: DocumentItemDto[]): number {
    return items.filter((item) => item.status === "draft").length;
  }

  function issuedCount(items: DocumentItemDto[]): number {
    return items.filter((item) => item.status !== "draft").length;
  }

  function nextAttentionLabel(items: DocumentItemDto[]): string {
    const draft = items.find((item) => item.status === "draft");
    return draft ? `${draft.number} · ${draft.counterparty}` : "Усі документи вже просунуті по сценарію";
  }
</script>

<section class="panel" data-testid="documents-screen">
  <div class="panel-header">
    <div>
      <h2>Документи</h2>
      <p>{$documents.list?.totalCount ?? 0} документів</p>
    </div>
    <input placeholder="Пошук документів" on:input={onDocumentSearch} />
  </div>

  <div class="create-strip-card" data-testid="documents-create-strip">
    <div class="create-strip-header">
      <div>
        <strong>Новий документ</strong>
        <p>1. Оберіть контрагента  2. Вкажіть тип документа  3. Створіть чернетку</p>
      </div>
      <span class="doc-kind-badge">
        <AppIcon name={documentKindIcons[createKind]} size={14} />
        <span>{documentKindLabels[createKind]}</span>
      </span>
    </div>

    <div class="create-strip">
      <label class="create-strip-field">
        <span>Контрагент</span>
        <select bind:value={createCounterpartyId}>
          <option value="">— Оберіть контрагента —</option>
          {#each $counterparties.screen?.items ?? [] as cp}
            <option value={cp.id}>{cp.name}</option>
          {/each}
        </select>
      </label>

      <label class="create-strip-field">
        <span>Тип документа</span>
        <select bind:value={createKind}>
          <option value="act">Акт</option>
          <option value="invoice">Рахунок</option>
          <option value="waybill">Накладна</option>
        </select>
      </label>

      <button class="btn-primary create-doc-button" disabled={!createCounterpartyId} on:click={onCreateDraft}>
        <AppIcon name={documentKindIcons[createKind]} surface={true} />
        <span>Створити чернетку</span>
      </button>
    </div>

    <p class="create-strip-hint">{getCreateHint(createCounterpartyId, createKind)}</p>
  </div>

  <div class="documents-focus-grid">
    <div class="documents-focus-card" data-testid="documents-focus-primary">
      <span class="reports-focus-label">Що потребує уваги</span>
      <strong>{draftCount($documents.list?.items ?? [])}</strong>
      <p>Чернетки, які ще не пройшли далі по сценарію й можуть затримати оплату або відвантаження.</p>
      <small>{nextAttentionLabel($documents.list?.items ?? [])}</small>
    </div>
    <div class="documents-focus-card documents-focus-card-muted">
      <span class="reports-focus-label">В роботі</span>
      <strong>{issuedCount($documents.list?.items ?? [])}</strong>
      <p>Документи, які вже виставлені або рухаються далі по ланцюжку.</p>
      <small>Виберіть рядок, щоб одразу перейти до редактора та наступної дії.</small>
    </div>
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

    <button class="btn-secondary" disabled={$documents.selectedIds.length === 0} on:click={onBulkAdvanceStatus}>
      Оновити статус вибраних
    </button>

    <button class="btn-danger" disabled={$documents.selectedIds.length === 0} on:click={onBulkDelete}>
      Видалити вибрані
    </button>
  </div>

  {#if $documents.message}
    <p class="message">{$documents.message}</p>
  {/if}

  {#if $documents.error}
    <p class="error">{$documents.error}</p>
  {/if}

  {#if ($documents.list?.items.length ?? 0) === 0}
    <div class="empty-state-card" data-testid="documents-empty-state">
      <strong>Поки що документів немає</strong>
      <p>Створіть першу чернетку, щоб запустити сценарій рахунку, акту або накладної.</p>
    </div>
  {:else}
    <div class="documents-list" data-testid="documents-list">
      {#each $documents.list?.items ?? [] as item}
        <button class="doc-row doc-row-rich" on:click={() => documents.open(item.id)}>
          <label class="doc-row-checkbox" aria-label={`Вибрати ${item.number}`}>
            <input
              type="checkbox"
              checked={$documents.selectedIds.includes(item.id)}
              on:click|stopPropagation={() => onToggleSelection(item.id)}
            />
          </label>

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
              <span>{item.amountStr}</span>
              <span class="doc-kind-badge">
                <AppIcon name={getDocumentKindIcon(item.kind)} size={14} />
                <span>{getDocumentKindLabel(item.kind)}</span>
              </span>
              <span class="doc-status-chip">{item.statusLabel}</span>
            </div>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</section>

{#if $documents.editor}
  <section class="editor-sheet">
    <div class="editor-header">
      <div>
        <h3>{$documents.editor.form.title}</h3>
        <p>{$documents.editor.form.counterpartyName}</p>
      </div>
      <div class="editor-actions">
        <button class="btn-ghost" on:click={() => documents.addItem()}>Додати позицію</button>
        <button class="btn-primary" on:click={() => documents.save()}>Зберегти</button>
        <button class="btn-secondary" on:click={() => documents.advanceStatus()}>Наступний статус</button>
        {#if ["act", "invoice"].includes($documents.editor.form.kind)}
          <button class="btn-secondary" on:click={() => documents.generatePdf()}>PDF</button>
        {/if}
        <button class="btn-danger" on:click={onDeleteCurrent}>Видалити</button>
        <button class="btn-ghost" on:click={() => documents.closeEditor()}>Закрити</button>
      </div>
    </div>

    <div class="editor-grid">
      <label>
        Номер
        <input value={$documents.editor.form.number} on:input={onEditorNumberChange} />
      </label>
      <label>
        Дата
        <input type="date" value={$documents.editor.form.date} on:input={onEditorDateChange} />
      </label>
      <label class="editor-grid-span">
        Примітки
        <textarea rows="3" value={$documents.editor.form.notes} on:input={onEditorNotesChange}></textarea>
      </label>
    </div>

    <div class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>Що далі</strong>
          <p>Переходьте до наступного документа без пошуку по інших екранах.</p>
        </div>
        <div class="chain-summary">
          <div class="chain-summary-block">
            <span>Поточний документ</span>
            <strong>{getDocumentKindLabel($documents.editor.form.kind)}</strong>
          </div>
          <div class="chain-summary-block">
            <span>Наступний крок</span>
            <strong>{getNextStepMessage($documents.editor.form.kind)}</strong>
          </div>
        </div>
      </div>

      <div class="chain-actions">
        <button class="btn-ghost" on:click={onReloadChain}>Оновити</button>
        {#each getChainTargets($documents.editor.form.kind) as targetKind}
          <button class="btn-secondary chain-action-button" on:click={() => onCreateChainDraft(targetKind)}>
            <AppIcon name={documentKindIcons[targetKind]} size={16} />
            <span>Створити {documentKindActionLabels[targetKind]}</span>
          </button>
        {/each}
      </div>

      {#if $documents.chain}
        <div class="chain-steps">
          {#each $documents.chain.steps as step}
            <div class:missing={!step.exists} class="chain-step">
              <div>
                <strong class="chain-doc-title">
                  <AppIcon name={getDocumentKindIcon(step.docType)} surface={true} size={16} />
                  <span>{getDocumentKindLabel(step.docType)}</span>
                </strong>
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

    <div class="editor-items-card">
      <div class="editor-items-header">
        <div>
          <strong>Позиції документа</strong>
          <p>Додайте товари або послуги, щоб документ одразу мав зрозумілу суму й склад.</p>
        </div>
        <span class="editor-items-count">{getItemsCountLabel($documents.editor.items.length)}</span>
      </div>

      <div class="editor-items">
        {#if $documents.editor.items.length === 0}
          <div class="editor-items-empty">
            <strong>Поки що без позицій</strong>
            <p>Додайте перший рядок, щоб заповнити номенклатуру, кількість та ціну в одному місці.</p>
            <button class="btn-secondary" on:click={() => documents.addItem()}>Додати першу позицію</button>
          </div>
        {:else}
          <div class="editor-item editor-item-head">
            <span>Опис</span>
            <span>Од.</span>
            <span>Кількість</span>
            <span>Ціна, грн</span>
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
                    placeholder="Опис"
                    on:input={(event) => onItemFieldChange(index, "description", event)}
                  />
                </label>
                <label class="editor-item-field">
                  <span>Од.</span>
                  <input value={item.unit} placeholder="Од." on:input={(event) => onItemFieldChange(index, "unit", event)} />
                </label>
                <label class="editor-item-field">
                  <span>Кількість</span>
                  <input
                    value={item.quantity}
                    placeholder="Кількість"
                    on:input={(event) => onItemFieldChange(index, "quantity", event)}
                  />
                </label>
                <label class="editor-item-field">
                  <span>Ціна, грн</span>
                  <input
                    value={item.price}
                    placeholder="Ціна"
                    on:input={(event) => onItemFieldChange(index, "price", event)}
                  />
                </label>
                <button class="btn-danger editor-item-remove" on:click={() => documents.removeItem(index)}>
                  Видалити позицію
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </section>
{/if}
