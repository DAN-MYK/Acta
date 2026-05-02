<script lang="ts">
  import AppIcon from "../components/AppIcon.svelte";
  import SkeletonCard from "../components/SkeletonCard.svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import type { AppIconName } from "../icons";
  import { documentsStore } from "../stores/documents";
  import { counterpartiesStore } from "../stores/counterparties";
  import type { DocumentDraftItemDto, DocumentKind, DocumentItemDto } from "../types";

  const documents = documentsStore;
  const counterparties = counterpartiesStore;

  let createCounterpartyId = "";
  let createKind: DocumentKind = "act";
  let selectedCounterpartyName = "";
  let lastDraftContextCounterpartyId = "";
  let createButton: HTMLButtonElement | null = null;
  let createCounterpartySelect: HTMLSelectElement | null = null;

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

  function getCreateHint(counterpartyId: string, kind: DocumentKind): string {
    if (!counterpartyId) {
      return "Спочатку оберіть контрагента, щоб ми відкрили чернетку в правильному робочому контексті.";
    }

    return `Чернетка типу "${documentKindLabels[kind]}" відкриється одразу для ${selectedCounterpartyName || "вибраного контрагента"} з готовим сценарієм подальших кроків.`;
  }

  function getNextStepMessage(kind: string): string {
    if (kind === "invoice") {
      return "Після рахунку зазвичай готуємо акт або накладну, щоб сценарій не зупинився на виставленні.";
    }
    if (kind === "act") {
      return "Після акта найчастіше залишилось або створити накладну, або перевести документ у наступний статус.";
    }
    if (kind === "waybill") {
      return "Накладна зазвичай закриває операційний сценарій, тому далі достатньо лише перевірити статус і суму.";
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

  function formatEditorDateValue(value: string): string {
    if (!value) {
      return "Оберіть дату через календар, щоб уникнути неоднозначного формату.";
    }

    const [year, month, day] = value.split("-");
    if (!year || !month || !day) {
      return "Оберіть дату через календар, щоб уникнути неоднозначного формату.";
    }

    return `${day}.${month}.${year}`;
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

  <div class="create-strip-card" data-testid="documents-create-strip">
    <div class="create-strip-header">
      <div>
        <strong>Створити документ по сценарію</strong>
        <p>Оберіть контрагента, визначте тип документа й одразу відкрийте чернетку з правильним наступним кроком.</p>
      </div>
      <span class="doc-kind-badge">
        <AppIcon name={documentKindIcons[createKind]} size={14} />
        <span>{documentKindLabels[createKind]}</span>
      </span>
    </div>

    <div class="create-strip-flow" aria-label="Сценарій створення документа">
      <div class:complete={!!createCounterpartyId} class="create-strip-step">
        <span>Крок 1</span>
        <strong>Контрагент</strong>
        <small>{selectedCounterpartyName || "Ще не обрано"}</small>
      </div>
      <div class="create-strip-step complete">
        <span>Крок 2</span>
        <strong>Тип документа</strong>
        <small>{documentKindLabels[createKind]}</small>
      </div>
      <div class:complete={!!createCounterpartyId} class="create-strip-step">
        <span>Крок 3</span>
        <strong>Чернетка</strong>
        <small>{createCounterpartyId ? "Можна відкривати редактор" : "Потрібен контрагент"}</small>
      </div>
    </div>

    <div class="create-strip">
      <label class="create-strip-field">
        <span>Контрагент</span>
        <select bind:this={createCounterpartySelect} bind:value={createCounterpartyId} disabled={$documents.loading}>
          <option value="">— Оберіть контрагента —</option>
          {#each $counterparties.screen?.items ?? [] as cp}
            <option value={cp.id}>{cp.name}</option>
          {/each}
        </select>
      </label>

      <label class="create-strip-field">
        <span>Тип документа</span>
        <select bind:value={createKind} disabled={$documents.loading}>
          <option value="act">Акт</option>
          <option value="invoice">Рахунок</option>
          <option value="waybill">Накладна</option>
        </select>
      </label>

        <button
          bind:this={createButton}
          class="btn-primary create-doc-button"
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

    <p class="create-strip-hint">{getCreateHint(createCounterpartyId, createKind)}</p>
  </div>

  <div class="documents-focus-grid">
    {#if $documents.initialLoading}
      <SkeletonCard count={2} />
    {:else}
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
    {/if}
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
    <p class="message">{$documents.message}</p>
  {/if}

  {#if $documents.error}
    <p class="error">{$documents.error}</p>
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
        <small class="field-note">Календар без ручного неоднозначного формату. Зараз: {formatEditorDateValue($documents.editor.form.date)}</small>
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

    <div class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>Статус і навігація по сценарію</strong>
          <p>Тут видно, де ви зараз у ланцюжку документа і який наступний крок можна зробити без переходів по інших екранах.</p>
        </div>
        <div class="chain-actions">
          <button class="btn-ghost" on:click={onReloadChain} disabled={$documents.loading}>Оновити</button>
          {#each getChainTargets($documents.editor.form.kind) as targetKind}
            <button
              class="btn-secondary chain-action-button"
              data-testid={`documents-chain-create-${targetKind}`}
              on:click={() => onCreateChainDraft(targetKind)}
              disabled={$documents.loading}
            >
              <AppIcon name={documentKindIcons[targetKind]} size={16} />
              <span>Створити {documentKindActionLabels[targetKind]}</span>
            </button>
          {/each}
        </div>
      </div>

      <div class="chain-summary">
        <div class="chain-summary-block">
          <span>Поточний документ</span>
          <strong>{getDocumentKindLabel($documents.editor.form.kind)}</strong>
        </div>
        <div class="chain-summary-block chain-summary-block-wide">
          <span>Наступний крок</span>
          <strong>{getNextStepMessage($documents.editor.form.kind)}</strong>
        </div>
      </div>

      {#if $documents.chain}
        <div class="chain-flow" data-testid="documents-chain-flow">
          {#each $documents.chain.steps as step, index}
            <div class:missing={!step.exists} class="chain-step-card">
              <span class="chain-step-index">Крок {index + 1}</span>
              <strong class="chain-doc-title">
                <AppIcon name={getDocumentKindIcon(step.docType)} surface={true} size={16} />
                <span>{getDocumentKindLabel(step.docType)}</span>
              </strong>
              <p>{step.docNumber || "Ще не створено"}</p>
              <div class="chain-step-meta">
                <span>{step.amountStr || "Сума з’явиться після створення"}</span>
                <span class="doc-status-chip">{step.status}</span>
              </div>
            </div>
          {/each}

          {#each getChainTargets($documents.editor.form.kind) as targetKind}
            <div class="chain-step-card chain-step-card-target">
              <span class="chain-step-index">Далі</span>
              <strong class="chain-doc-title">
                <AppIcon name={documentKindIcons[targetKind]} surface={true} size={16} />
                <span>{getDocumentKindLabel(targetKind)}</span>
              </strong>
              <p>Ще не створено</p>
              <div class="chain-step-meta">
                <span>Підготуйте наступний документ прямо з цього блоку.</span>
                <span class="doc-status-chip">Очікує дії</span>
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
          <p>Додавайте товари або послуги так, щоб сума й склад документа читалися з першого погляду.</p>
        </div>
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
  </section>
{/if}
