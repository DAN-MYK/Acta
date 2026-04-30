# Svelte Screens Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Розбити `frontend/src/App.svelte` (1386 рядків) на 8 окремих файлів: тонка оболонка `App.svelte` + 7 screen-компонентів у `frontend/src/screens/`, без жодних змін логіки чи дизайну.

**Architecture:** Store-bound screens — кожен screen-компонент сам імпортує потрібні stores, читає реактивно через `$store`, викликає store-методи напряму. App.svelte залишається оболонкою: sidebar, topbar, palette, onMount, onCompanyChange, маршрутизація `{#if currentScreen === "..."}<Screen />{/if}`.

**Tech Stack:** Svelte 4, TypeScript, Vite. Перевірка: `cd frontend && npm run check` (svelte-check). Тести: `cd frontend && npm run test:frontend` (Vitest).

---

## File Map

| Дія | Файл |
|-----|------|
| Modify | `frontend/src/App.svelte` — зменшити до ~120 рядків |
| Create | `frontend/src/screens/Dashboard.svelte` |
| Create | `frontend/src/screens/Documents.svelte` |
| Create | `frontend/src/screens/Counterparties.svelte` |
| Create | `frontend/src/screens/Payments.svelte` |
| Create | `frontend/src/screens/Reports.svelte` |
| Create | `frontend/src/screens/Tasks.svelte` |
| Create | `frontend/src/screens/Settings.svelte` |
| Modify | `.gitignore` — додати `.superpowers/` |

---

## Task 1: Housekeeping + Dashboard.svelte

**Files:**
- Modify: `.gitignore`
- Create: `frontend/src/screens/Dashboard.svelte`

- [ ] **Step 1: Додати `.superpowers/` до `.gitignore`**

Відкрий `.gitignore` і додай рядок в кінець:

```
.superpowers/
```

- [ ] **Step 2: Створити `frontend/src/screens/Dashboard.svelte`**

```svelte
<script lang="ts">
  import { dashboardStore } from "../lib/stores/dashboard";
  import { documentsStore } from "../lib/stores/documents";
  import { navigationStore } from "../lib/stores/navigation";
  import { tasksStore } from "../lib/stores/tasks";

  const dashboard = dashboardStore;
  const navigation = navigationStore;
  const documents = documentsStore;
  const tasks = tasksStore;

  function openDashboardDocument(docId: string) {
    navigation.go("documents");
    void documents.open(docId);
  }

  function openDashboardTask(taskId: string) {
    navigation.go("tasks");
    void tasks.openEditor(taskId);
  }
</script>

<section class="panel dashboard-panel">
  <div class="panel-header">
    <div>
      <h2>Дашборд</h2>
      <p>Операційна картина по активній компанії</p>
    </div>
    <button on:click={() => dashboard.load()} disabled={$dashboard.loading}>
      {$dashboard.loading ? "Оновлення..." : "Оновити"}
    </button>
  </div>

  {#if $dashboard.error}
    <p class="error">{$dashboard.error}</p>
  {/if}

  <div class="dashboard-kpis">
    {#each $dashboard.screen?.kpis ?? [] as kpi}
      <article
        class="dashboard-kpi-card"
        class:positive={kpi.tone === "positive"}
        class:warning={kpi.tone === "warning"}
        class:danger={kpi.tone === "danger"}
      >
        <span>{kpi.label}</span>
        <strong>{kpi.value}</strong>
        <small>{kpi.detail}</small>
      </article>
    {/each}
  </div>

  <div class="dashboard-grid">
    <article class="dashboard-card wide">
      <div class="card-title">
        <h3>Грошовий потік</h3>
        <span>Останні 90 днів</span>
      </div>
      <div class="cashflow-list">
        {#each $dashboard.screen?.cashflowRows ?? [] as row}
          <div class="cashflow-row">
            <div>
              <strong>{row.label}</strong>
              <span>{row.netStr}</span>
            </div>
            <div class="cashflow-bars">
              <span class="income">{row.incomeStr}</span>
              <span class="expense">{row.expenseStr}</span>
            </div>
          </div>
        {/each}
      </div>
    </article>

    <article class="dashboard-card">
      <div class="card-title">
        <h3>Останні документи</h3>
        <button on:click={() => navigation.go("documents")}>Відкрити</button>
      </div>
      {#each $dashboard.screen?.recentDocuments ?? [] as doc}
        <button class="dashboard-list-row" on:click={() => openDashboardDocument(doc.id)}>
          <span>{doc.number} · {doc.counterparty}</span>
          <strong>{doc.amountStr}</strong>
        </button>
      {/each}
    </article>

    <article class="dashboard-card">
      <div class="card-title">
        <h3>Завдання у фокусі</h3>
        <button on:click={() => navigation.go("tasks")}>Відкрити</button>
      </div>
      {#each $dashboard.screen?.urgentTasks ?? [] as task}
        <button class="dashboard-list-row" on:click={() => openDashboardTask(task.id)}>
          <span>{task.title}</span>
          <strong>{task.dueDate || task.priorityLabel}</strong>
        </button>
      {/each}
    </article>
  </div>
</section>
```

- [ ] **Step 3: Перевірити TypeScript**

```bash
cd frontend && npm run check
```

Очікується: `svelte-check` завершується без помилок.

- [ ] **Step 4: Commit**

```bash
git add .gitignore frontend/src/screens/Dashboard.svelte
git commit -m "feat(screens): add Dashboard screen component"
```

---

## Task 2: Documents.svelte

**Files:**
- Create: `frontend/src/screens/Documents.svelte`

- [ ] **Step 1: Створити `frontend/src/screens/Documents.svelte`**

```svelte
<script lang="ts">
  import { documentsStore } from "../lib/stores/documents";
  import type { DocumentKind } from "../lib/types";

  const documents = documentsStore;

  let createCounterpartyId = "";
  let createKind: DocumentKind = "act";

  const chainTargetLabels: Record<DocumentKind, string> = {
    invoice: "Рахунок",
    act: "Акт",
    waybill: "Накладна"
  };

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

  function getChainTargets(kind: string): DocumentKind[] {
    if (kind === "invoice") return ["act", "waybill"];
    if (kind === "act") return ["waybill"];
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
```

- [ ] **Step 2: Перевірити TypeScript**

```bash
cd frontend && npm run check
```

Очікується: без помилок.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/screens/Documents.svelte
git commit -m "feat(screens): add Documents screen component"
```

---

## Task 3: Counterparties.svelte

**Files:**
- Create: `frontend/src/screens/Counterparties.svelte`

- [ ] **Step 1: Створити `frontend/src/screens/Counterparties.svelte`**

```svelte
<script lang="ts">
  import { counterpartiesStore } from "../lib/stores/counterparties";
  import { documentsStore } from "../lib/stores/documents";
  import { navigationStore } from "../lib/stores/navigation";

  const counterparties = counterpartiesStore;
  const navigation = navigationStore;
  const documents = documentsStore;

  function onCounterpartySearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void counterparties.load(input.value);
  }

  function onCounterpartyFieldChange(field: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLTextAreaElement;
    counterparties.updateFormField(
      field as "name" | "edrpou" | "ipn" | "iban" | "address" | "phone" | "email" | "notes",
      input.value
    );
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Контрагенти</h2>
      <p>{$counterparties.screen?.items.length ?? 0} записів</p>
    </div>
    <div class="panel-actions">
      <input placeholder="Пошук контрагентів" on:input={onCounterpartySearch} />
      <button on:click={() => counterparties.openEditor()}>Новий контрагент</button>
    </div>
  </div>

  {#if $counterparties.message}
    <p class="message">{$counterparties.message}</p>
  {/if}

  {#if $counterparties.error}
    <p class="error">{$counterparties.error}</p>
  {/if}

  <div class="counterparties-layout">
    <div class="counterparties-list">
      {#each $counterparties.screen?.items ?? [] as item}
        <button
          class="counterparty-row"
          class:active={$counterparties.selectedId === item.id}
          on:click={() => counterparties.open(item.id)}
        >
          <div>
            <strong>{item.name}</strong>
            <p>{item.edrpou || "Без ЄДРПОУ"}</p>
          </div>
        </button>
      {/each}
    </div>

    <div class="counterparty-detail">
      {#if $counterparties.detail}
        <div class="counterparty-detail-header">
          <div>
            <h3>{$counterparties.detail.info.name}</h3>
            <p>{$counterparties.detail.info.edrpou || "Без ЄДРПОУ"}</p>
          </div>
          <div class="editor-actions">
            <button on:click={() => counterparties.openEditor($counterparties.detail?.info.id)}>
              Редагувати
            </button>
            <button on:click={() => counterparties.createDocument()}>Створити документ</button>
            <button class="ghost-danger" on:click={() => counterparties.archiveCurrent()}>
              Архівувати
            </button>
          </div>
        </div>

        <div class="detail-grid">
          <div>
            <strong>IBAN</strong>
            <p>{$counterparties.detail.info.iban || "—"}</p>
          </div>
          <div>
            <strong>Телефон</strong>
            <p>{$counterparties.detail.info.phone || "—"}</p>
          </div>
          <div>
            <strong>Email</strong>
            <p>{$counterparties.detail.info.email || "—"}</p>
          </div>
          <div>
            <strong>Адреса</strong>
            <p>{$counterparties.detail.info.address || "—"}</p>
          </div>
        </div>

        <div class="linked-block">
          <strong>Документи</strong>
          <div class="linked-list">
            {#each $counterparties.detail.documents as item}
              <button
                class="linked-row"
                on:click={() => {
                  navigation.go("documents");
                  void documents.open(item.id);
                }}
              >
                <span>{item.number}</span>
                <span>{item.amountStr}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="linked-block">
          <strong>Платежі</strong>
          <div class="linked-list">
            {#each $counterparties.detail.payments as payment}
              <div class="linked-row static">
                <span>{payment.date}</span>
                <span>{payment.amountStr}</span>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="empty-screen compact">
          <p>Виберіть контрагента зі списку.</p>
        </div>
      {/if}
    </div>
  </div>
</section>

{#if $counterparties.editor}
  <section class="editor-sheet">
    <div class="editor-header">
      <div>
        <h3>{$counterparties.editor.form.title}</h3>
        <p>Картка контрагента</p>
      </div>
      <div class="editor-actions">
        <button on:click={() => counterparties.save()}>Зберегти</button>
        <button on:click={() => counterparties.closeEditor()}>Закрити</button>
      </div>
    </div>

    <div class="editor-grid cp-editor-grid">
      <label>
        Назва
        <input value={$counterparties.editor.form.name} on:input={(event) => onCounterpartyFieldChange("name", event)} />
      </label>
      <label>
        ЄДРПОУ
        <input value={$counterparties.editor.form.edrpou} on:input={(event) => onCounterpartyFieldChange("edrpou", event)} />
      </label>
      <label>
        ІПН
        <input value={$counterparties.editor.form.ipn} on:input={(event) => onCounterpartyFieldChange("ipn", event)} />
      </label>
      <label>
        IBAN
        <input value={$counterparties.editor.form.iban} on:input={(event) => onCounterpartyFieldChange("iban", event)} />
      </label>
      <label>
        Телефон
        <input value={$counterparties.editor.form.phone} on:input={(event) => onCounterpartyFieldChange("phone", event)} />
      </label>
      <label>
        Email
        <input value={$counterparties.editor.form.email} on:input={(event) => onCounterpartyFieldChange("email", event)} />
      </label>
      <label class="editor-grid-span">
        Адреса
        <input value={$counterparties.editor.form.address} on:input={(event) => onCounterpartyFieldChange("address", event)} />
      </label>
      <label class="editor-grid-span">
        Примітки
        <textarea rows="4" value={$counterparties.editor.form.notes} on:input={(event) => onCounterpartyFieldChange("notes", event)}></textarea>
      </label>
    </div>
  </section>
{/if}
```

- [ ] **Step 2: Перевірити TypeScript**

```bash
cd frontend && npm run check
```

Очікується: без помилок.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/screens/Counterparties.svelte
git commit -m "feat(screens): add Counterparties screen component"
```

---

## Task 4: Payments.svelte

**Files:**
- Create: `frontend/src/screens/Payments.svelte`

- [ ] **Step 1: Створити `frontend/src/screens/Payments.svelte`**

```svelte
<script lang="ts">
  import { paymentsStore } from "../lib/stores/payments";
  import type { PaymentDraftFormDto, PaymentItemDto } from "../lib/types";

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
            <strong>{item.date} — {item.counterparty || "Без контрагента"}</strong>
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
          <option value="">— Без контрагента —</option>
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
```

- [ ] **Step 2: Перевірити TypeScript**

```bash
cd frontend && npm run check
```

Очікується: без помилок.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/screens/Payments.svelte
git commit -m "feat(screens): add Payments screen component"
```

---

## Task 5: Reports.svelte

**Files:**
- Create: `frontend/src/screens/Reports.svelte`

- [ ] **Step 1: Створити `frontend/src/screens/Reports.svelte`**

```svelte
<script lang="ts">
  import { reportsStore } from "../lib/stores/reports";
  import type { ReportsScope, ReportsTab } from "../lib/types";

  const reports = reportsStore;

  function onReportsSearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void reports.load({ query: input.value });
  }

  function onReportsTabChange(tab: ReportsTab) {
    void reports.load({ tab });
  }

  function onReportsScopeChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    void reports.load({ scope: select.value as ReportsScope });
  }

  function onReportsDateChange(field: "dateFrom" | "dateTo", event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void reports.load({ [field]: input.value });
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Звіти</h2>
      <p>Bank / receivables / payables у Tauri runtime</p>
    </div>
    <div class="panel-actions">
      <input
        placeholder="Пошук по поточному звіту"
        value={$reports.screen?.filter.query ?? ""}
        on:input={onReportsSearch}
      />
      <button on:click={() => reports.exportCsv()}>Експорт CSV</button>
    </div>
  </div>

  <div class="reports-filters">
    <div class="task-tabs">
      <button class:active={$reports.screen?.filter.tab === "bank"} on:click={() => onReportsTabChange("bank")}>
        Банк
      </button>
      <button class:active={$reports.screen?.filter.tab === "receivables"} on:click={() => onReportsTabChange("receivables")}>
        Дебіторка
      </button>
      <button class:active={$reports.screen?.filter.tab === "payables"} on:click={() => onReportsTabChange("payables")}>
        Кредиторка
      </button>
    </div>

    <div class="reports-filter-grid">
      <label>
        Scope
        <select value={$reports.screen?.filter.scope ?? "active"} on:change={onReportsScopeChange}>
          <option value="active">Активна компанія</option>
          <option value="all">Усі компанії</option>
        </select>
      </label>
      <label>
        Дата від
        <input type="date" value={$reports.screen?.filter.dateFrom ?? ""} on:input={(event) => onReportsDateChange("dateFrom", event)} />
      </label>
      <label>
        Дата до
        <input type="date" value={$reports.screen?.filter.dateTo ?? ""} on:input={(event) => onReportsDateChange("dateTo", event)} />
      </label>
    </div>
  </div>

  <div class="reports-kpis">
    <div class="task-kpi-card">
      <strong>{$reports.screen?.summary.openingBalanceStr ?? "0,00 грн"}</strong>
      <span>Залишок на початок</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$reports.screen?.summary.incomeStr ?? "0,00 грн"}</strong>
      <span>Надходження</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$reports.screen?.summary.receivablesTotalStr ?? "0,00 грн"}</strong>
      <span>Дебіторка</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$reports.screen?.summary.payablesTotalStr ?? "0,00 грн"}</strong>
      <span>Кредиторка</span>
    </div>
  </div>

  {#if $reports.message}
    <p class="message">{$reports.message}</p>
  {/if}

  {#if $reports.error}
    <p class="error">{$reports.error}</p>
  {/if}

  {#if $reports.screen?.filter.tab === "bank"}
    <div class="reports-table">
      <div class="reports-table-row reports-table-head">
        <span>Група</span>
        <span>Надходження</span>
        <span>Витрати</span>
        <span>Net</span>
      </div>
      {#each $reports.screen?.bankRows ?? [] as row}
        <div class="reports-table-row">
          <span>{row.label}</span>
          <span>{row.incomeStr}</span>
          <span>{row.expenseStr}</span>
          <span>{row.netStr}</span>
        </div>
      {/each}
    </div>
  {:else if $reports.screen?.filter.tab === "receivables"}
    <div class="reports-table">
      <div class="reports-table-row reports-table-head reports-table-wide">
        <span>Документ</span>
        <span>Дата</span>
        <span>Компанія</span>
        <span>Контрагент</span>
        <span>Сума</span>
        <span>Очікувана дата</span>
        <span>Прострочка</span>
      </div>
      {#each $reports.screen?.receivablesRows ?? [] as row}
        <div class="reports-table-row reports-table-wide">
          <span>{row.docNumber}</span>
          <span>{row.docDate}</span>
          <span>{row.companyName}</span>
          <span>{row.counterparty}</span>
          <span>{row.amountStr}</span>
          <span>{row.expectedDate || "—"}</span>
          <span>{row.overdueDays > 0 ? `${row.overdueDays} дн.` : "—"}</span>
        </div>
      {/each}
    </div>
  {:else}
    <div class="reports-table">
      <div class="reports-table-row reports-table-head reports-table-wide">
        <span>Назва</span>
        <span>Компанія</span>
        <span>Контрагент</span>
        <span>Сума</span>
        <span>Дата</span>
        <span>Прострочка</span>
        <span>Повтор</span>
      </div>
      {#each $reports.screen?.payablesRows ?? [] as row}
        <div class="reports-table-row reports-table-wide">
          <span>{row.title}</span>
          <span>{row.companyName}</span>
          <span>{row.counterparty || "—"}</span>
          <span>{row.amountStr}</span>
          <span>{row.dueDate}</span>
          <span>{row.overdueDays > 0 ? `${row.overdueDays} дн.` : "—"}</span>
          <span>{row.recurrence}</span>
        </div>
      {/each}
    </div>
  {/if}
</section>
```

- [ ] **Step 2: Перевірити TypeScript**

```bash
cd frontend && npm run check
```

Очікується: без помилок.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/screens/Reports.svelte
git commit -m "feat(screens): add Reports screen component"
```

---

## Task 6: Tasks.svelte

**Files:**
- Create: `frontend/src/screens/Tasks.svelte`

- [ ] **Step 1: Створити `frontend/src/screens/Tasks.svelte`**

```svelte
<script lang="ts">
  import { tasksStore } from "../lib/stores/tasks";
  import type { TaskDraftFormDto, TaskItemDto, TaskStatus } from "../lib/types";

  const tasks = tasksStore;

  function onTaskSearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void tasks.load(input.value);
  }

  function onTaskFieldChange(field: keyof TaskDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
    tasks.updateFormField(field, input.value);
  }

  function taskItemsForTab(items: TaskItemDto[], tab: "open" | "done" | "all") {
    if (tab === "done") return items.filter((item) => item.status === "done" || item.status === "cancelled");
    if (tab === "all") return items;
    return items.filter((item) => item.status === "open" || item.status === "in_progress");
  }

  function todayTaskItems(items: TaskItemDto[]) {
    const today = new Date().toISOString().slice(0, 10);
    return items.filter((item) => item.dueDate === today || item.reminderAt.startsWith(today));
  }

  function toggleTaskStatus(task: TaskItemDto) {
    const nextStatus: TaskStatus = task.status === "done" ? "open" : "done";
    void tasks.setStatus(task.id, nextStatus);
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Завдання</h2>
      <p>{$tasks.screen?.items.length ?? 0} записів у поточній вибірці</p>
    </div>
    <div class="panel-actions">
      <input placeholder="Пошук завдань" on:input={onTaskSearch} />
      <button on:click={() => tasks.openEditor()}>Нове завдання</button>
    </div>
  </div>

  <div class="task-kpis">
    <div class="task-kpi-card">
      <strong>{$tasks.screen?.openCount ?? 0}</strong>
      <span>Активні</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$tasks.screen?.doneCount ?? 0}</strong>
      <span>Завершені</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$tasks.screen?.highCount ?? 0}</strong>
      <span>Високий пріоритет</span>
    </div>
    <div class="task-kpi-card">
      <strong>{$tasks.screen?.todayCount ?? 0}</strong>
      <span>На сьогодні</span>
    </div>
  </div>

  {#if $tasks.message}
    <p class="message">{$tasks.message}</p>
  {/if}

  {#if $tasks.error}
    <p class="error">{$tasks.error}</p>
  {/if}

  <div class="tasks-layout">
    <div class="tasks-main">
      <div class="task-tabs">
        <button class:active={$tasks.tab === "open"} on:click={() => tasks.setTab("open")}>Активні</button>
        <button class:active={$tasks.tab === "done"} on:click={() => tasks.setTab("done")}>Завершені</button>
        <button class:active={$tasks.tab === "all"} on:click={() => tasks.setTab("all")}>Усі</button>
      </div>

      <div class="tasks-list">
        {#each taskItemsForTab($tasks.screen?.items ?? [], $tasks.tab) as item}
          <div class="task-row">
            <button class="task-row-main" on:click={() => tasks.openEditor(item.id)}>
              <div>
                <strong>{item.title}</strong>
                <p>{item.description || item.priorityLabel}</p>
              </div>
              <div class="task-row-meta">
                <span class="task-pill">{item.priorityLabel}</span>
                <span>{item.dueDate || "Без дедлайну"}</span>
                <span>{item.statusLabel}</span>
              </div>
            </button>
            <button on:click={() => toggleTaskStatus(item)}>
              {item.status === "done" ? "Повернути" : "Готово"}
            </button>
          </div>
        {/each}
      </div>
    </div>

    <aside class="tasks-side-panel">
      <strong>Сьогодні</strong>
      <div class="linked-list">
        {#each todayTaskItems($tasks.screen?.items ?? []) as item}
          <button class="linked-row" on:click={() => tasks.openEditor(item.id)}>
            <span>{item.title}</span>
            <span>{item.reminderAt || item.dueDate}</span>
          </button>
        {/each}
      </div>
    </aside>
  </div>
</section>

{#if $tasks.editor}
  <section class="editor-sheet">
    <div class="editor-header">
      <div>
        <h3>{$tasks.editor.title}</h3>
        <p>{$tasks.editor.form.linkLabel || "Без прив'язки"}</p>
      </div>
      <div class="editor-actions">
        <button on:click={() => tasks.save()}>Зберегти</button>
        {#if $tasks.editor.form.id}
          <button class="ghost-danger" on:click={() => tasks.deleteCurrent()}>Видалити</button>
        {/if}
        <button on:click={() => tasks.closeEditor()}>Закрити</button>
      </div>
    </div>

    <div class="editor-grid">
      <label class="editor-grid-span">
        Назва
        <input value={$tasks.editor.form.title} on:input={(event) => onTaskFieldChange("title", event)} />
      </label>
      <label class="editor-grid-span">
        Опис
        <textarea rows="4" value={$tasks.editor.form.description} on:input={(event) => onTaskFieldChange("description", event)}></textarea>
      </label>
      <label>
        Пріоритет
        <select value={$tasks.editor.form.priority} on:change={(event) => onTaskFieldChange("priority", event)}>
          <option value="low">Низький</option>
          <option value="normal">Звичайний</option>
          <option value="high">Високий</option>
          <option value="critical">Критичний</option>
        </select>
      </label>
      <label>
        Статус
        <select value={$tasks.editor.form.status} on:change={(event) => onTaskFieldChange("status", event)}>
          <option value="open">Відкрите</option>
          <option value="in_progress">В роботі</option>
          <option value="done">Виконано</option>
          <option value="cancelled">Скасовано</option>
        </select>
      </label>
      <label>
        Дедлайн
        <input type="date" value={$tasks.editor.form.dueDate} on:input={(event) => onTaskFieldChange("dueDate", event)} />
      </label>
      <label>
        Нагадування
        <input type="datetime-local" value={$tasks.editor.form.reminderAt} on:input={(event) => onTaskFieldChange("reminderAt", event)} />
      </label>
    </div>
  </section>
{/if}
```

- [ ] **Step 2: Перевірити TypeScript**

```bash
cd frontend && npm run check
```

Очікується: без помилок.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/screens/Tasks.svelte
git commit -m "feat(screens): add Tasks screen component"
```

---

## Task 7: Settings.svelte

**Files:**
- Create: `frontend/src/screens/Settings.svelte`

- [ ] **Step 1: Створити `frontend/src/screens/Settings.svelte`**

```svelte
<script lang="ts">
  import { settingsStore } from "../lib/stores/settings";
  import { shellStore } from "../lib/stores/shell";
  import { themeStore } from "../lib/stores/theme";
  import type { SettingsCompanyDto, SettingsSection } from "../lib/types";

  const settings = settingsStore;
  const shell = shellStore;
  const theme = themeStore;

  const settingsSections: Array<[SettingsSection, string]> = [
    ["appearance", "Зовнішній вигляд"],
    ["company", "Компанія"],
    ["numbering", "Нумерація"],
    ["integrations", "Інтеграції"],
    ["team", "Команда"],
    ["backup", "Резервні копії"]
  ];

  function onSettingsSectionChange(section: SettingsSection) {
    settings.setSection(section);
  }

  async function onSettingsThemeChange(darkMode: boolean) {
    theme.setMode(darkMode ? "dark" : "light");
    settings.updatePreference("darkMode", darkMode);
    await settings.savePreferences();
    await shell.load();
  }

  async function onSettingsDensityChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    settings.updatePreference("density", Number(select.value));
    await settings.savePreferences();
  }

  function onSettingsCompanyFieldChange(field: keyof SettingsCompanyDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const value = input.type === "checkbox" ? input.checked : input.value;
    settings.updateCompanyField(field, value);
  }

  async function onSettingsCompanySave() {
    const result = await settings.saveCompany();
    if (result) {
      await shell.load();
    }
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Налаштування</h2>
      <p>Tauri vertical slice для appearance, company, integrations, team та backup</p>
    </div>
  </div>

  <div class="settings-layout">
    <aside class="settings-nav">
      {#each settingsSections as [section, label]}
        <button class:active={$settings.section === section} on:click={() => onSettingsSectionChange(section)}>
          {label}
        </button>
      {/each}
    </aside>

    <div class="settings-content">
      {#if $settings.message}
        <p class="message">{$settings.message}</p>
      {/if}

      {#if $settings.error}
        <p class="error">{$settings.error}</p>
      {/if}

      {#if $settings.section === "appearance"}
        <div class="settings-card">
          <h3>Зовнішній вигляд</h3>
          <div class="settings-actions-row">
            <button class:active={!$settings.screen?.preferences.darkMode} on:click={() => onSettingsThemeChange(false)}>
              Світла тема
            </button>
            <button class:active={$settings.screen?.preferences.darkMode} on:click={() => onSettingsThemeChange(true)}>
              Темна тема
            </button>
            <select value={$settings.screen?.preferences.density ?? 1} on:change={onSettingsDensityChange}>
              <option value="0">Compact</option>
              <option value="1">Comfortable</option>
              <option value="2">Spacious</option>
            </select>
          </div>
        </div>
      {:else if $settings.section === "company"}
        <div class="settings-card">
          <div class="editor-header">
            <div>
              <h3>Компанія</h3>
              <p>{$settings.screen?.company.vatCert ?? ""}</p>
            </div>
            <div class="editor-actions">
              <button on:click={onSettingsCompanySave}>Зберегти</button>
            </div>
          </div>

          <div class="editor-grid cp-editor-grid">
            <label>
              Назва
              <input value={$settings.screen?.company.fullName ?? ""} on:input={(event) => onSettingsCompanyFieldChange("fullName", event)} />
            </label>
            <label>
              Коротка назва
              <input value={$settings.screen?.company.shortName ?? ""} on:input={(event) => onSettingsCompanyFieldChange("shortName", event)} />
            </label>
            <label>
              ЄДРПОУ
              <input value={$settings.screen?.company.edrpou ?? ""} on:input={(event) => onSettingsCompanyFieldChange("edrpou", event)} />
            </label>
            <label>
              ІПН
              <input value={$settings.screen?.company.ipn ?? ""} on:input={(event) => onSettingsCompanyFieldChange("ipn", event)} />
            </label>
            <label>
              IBAN
              <input value={$settings.screen?.company.iban ?? ""} on:input={(event) => onSettingsCompanyFieldChange("iban", event)} />
            </label>
            <label>
              Директор
              <input value={$settings.screen?.company.director ?? ""} on:input={(event) => onSettingsCompanyFieldChange("director", event)} />
            </label>
            <label class="editor-grid-span">
              Адреса
              <input value={$settings.screen?.company.address ?? ""} on:input={(event) => onSettingsCompanyFieldChange("address", event)} />
            </label>
            <label class="settings-checkbox">
              <input
                type="checkbox"
                checked={$settings.screen?.company.vatRegistered ?? false}
                on:change={(event) => onSettingsCompanyFieldChange("vatRegistered", event)}
              />
              Платник ПДВ
            </label>
          </div>
        </div>
      {:else if $settings.section === "numbering"}
        <div class="settings-card">
          <h3>Нумерація</h3>
          <div class="reports-table">
            <div class="reports-table-row reports-table-head reports-table-wide settings-numbering-row">
              <span>Тип</span>
              <span>Шаблон</span>
              <span>Приклад</span>
              <span>Наступний №</span>
            </div>
            {#each $settings.screen?.numbering ?? [] as row}
              <div class="reports-table-row reports-table-wide settings-numbering-row">
                <span>{row.docType}</span>
                <span>{row.template}</span>
                <span>{row.example}</span>
                <span>{row.nextNumber}</span>
              </div>
            {/each}
          </div>
        </div>
      {:else if $settings.section === "integrations"}
        <div class="settings-card">
          <h3>Інтеграції</h3>
          <div class="linked-list">
            {#each $settings.screen?.integrations ?? [] as integration}
              <div class="settings-row">
                <div>
                  <strong>{integration.label}</strong>
                  <p>{integration.description}</p>
                </div>
                <div class="settings-row-actions">
                  <span>{integration.enabled ? "Активно" : "Вимкнено"}</span>
                  <button on:click={() => settings.configureIntegration(integration.tag)}>
                    {integration.enabled ? "Налаштувати" : "Підключити"}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else if $settings.section === "team"}
        <div class="settings-card">
          <div class="editor-header">
            <div>
              <h3>Команда</h3>
              <p>{$settings.screen?.team.length ?? 0} користувачів</p>
            </div>
            <div class="editor-actions">
              <button on:click={() => settings.inviteTeam()}>Запросити</button>
            </div>
          </div>
          <div class="linked-list">
            {#each $settings.screen?.team ?? [] as member}
              <div class="settings-row">
                <div>
                  <strong>{member.name}</strong>
                  <p>{member.email}</p>
                </div>
                <div class="settings-row-actions">
                  <span>{member.role}</span>
                  <span>{member.lastActive}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="settings-card">
          <div class="editor-header">
            <div>
              <h3>Резервні копії</h3>
              <p>{$settings.screen?.backup.kind ?? ""}</p>
            </div>
            <div class="editor-actions">
              <button on:click={() => settings.openLatestBackup()}>Відкрити копію</button>
              <button on:click={() => settings.backupNow()}>Створити зараз</button>
            </div>
          </div>

          <div class="task-kpi-card">
            <strong>{$settings.screen?.backup.label ?? "—"}</strong>
            <span>{$settings.screen?.backup.file ?? ""}</span>
            <span>{$settings.screen?.backup.note ?? ""}</span>
          </div>
        </div>
      {/if}
    </div>
  </div>
</section>
```

- [ ] **Step 2: Перевірити TypeScript**

```bash
cd frontend && npm run check
```

Очікується: без помилок.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/screens/Settings.svelte
git commit -m "feat(screens): add Settings screen component"
```

---

## Task 8: Refactor App.svelte to thin shell

**Files:**
- Modify: `frontend/src/App.svelte` — замінити повністю

- [ ] **Step 1: Замінити вміст `frontend/src/App.svelte`**

```svelte
<script lang="ts">
  import { onMount, tick } from "svelte";
  import { counterpartiesStore } from "./lib/stores/counterparties";
  import { dashboardStore } from "./lib/stores/dashboard";
  import { documentsStore } from "./lib/stores/documents";
  import { navigationStore } from "./lib/stores/navigation";
  import { paletteStore } from "./lib/stores/palette";
  import { reportsStore } from "./lib/stores/reports";
  import { settingsStore } from "./lib/stores/settings";
  import { shellStore } from "./lib/stores/shell";
  import { tasksStore } from "./lib/stores/tasks";
  import { paymentsStore } from "./lib/stores/payments";
  import { themeStore } from "./lib/stores/theme";
  import type { ScreenId } from "./lib/types";
  import Dashboard from "./screens/Dashboard.svelte";
  import Documents from "./screens/Documents.svelte";
  import Counterparties from "./screens/Counterparties.svelte";
  import Payments from "./screens/Payments.svelte";
  import Reports from "./screens/Reports.svelte";
  import Tasks from "./screens/Tasks.svelte";
  import Settings from "./screens/Settings.svelte";

  const navigation = navigationStore;
  const shell = shellStore;
  const palette = paletteStore;
  const theme = themeStore;
  const settings = settingsStore;

  let paletteInput: HTMLInputElement | null = null;

  const sidebarScreens: Array<{ screen: ScreenId; label: string }> = [
    { screen: "dashboard", label: "Дашборд" },
    { screen: "documents", label: "Документи" },
    { screen: "counterparties", label: "Контрагенти" },
    { screen: "payments", label: "Платежі" },
    { screen: "reports", label: "Звіти" },
    { screen: "tasks", label: "Завдання" },
    { screen: "settings", label: "Налаштування" }
  ];

  onMount(async () => {
    await shell.load();
    const settingsScreen = await settings.load();
    if (settingsScreen) {
      theme.setMode(settingsScreen.preferences.darkMode ? "dark" : "light");
    }
    await Promise.all([
      dashboardStore.load(),
      documentsStore.load(),
      counterpartiesStore.load(),
      tasksStore.load(),
      reportsStore.load(),
      paymentsStore.load()
    ]);
  });

  async function onCompanyChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    await shell.setActiveCompany(select.value);
    const settingsScreen = await settings.load();
    if (settingsScreen) {
      theme.setMode(settingsScreen.preferences.darkMode ? "dark" : "light");
    }
    await Promise.all([
      dashboardStore.load(),
      documentsStore.load(),
      counterpartiesStore.load(),
      tasksStore.load(),
      reportsStore.load(),
      paymentsStore.load()
    ]);
  }

  function onPaletteInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void palette.search(input.value);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.key.toLowerCase() === "k") {
      event.preventDefault();
      palette.toggle();
    }
    if (event.ctrlKey && event.key >= "1" && event.key <= "7") {
      event.preventDefault();
      const screens: ScreenId[] = [
        "dashboard", "documents", "counterparties", "payments",
        "reports", "tasks", "settings"
      ];
      navigation.go(screens[Number(event.key) - 1]);
    }
  }

  $: if ($palette.open) {
    void tick().then(() => paletteInput?.focus());
  }

  $: document.body.dataset.theme = $theme;
  $: currentScreen = $navigation;
  $: shellState = $shell.state;
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">A</div>
      <div>
        <strong>Acta</strong>
        <p>Tauri migration scaffold</p>
      </div>
    </div>

    <nav class="nav">
      {#each sidebarScreens as item}
        <button class:active={currentScreen === item.screen} on:click={() => navigation.go(item.screen)}>
          {item.label}
        </button>
      {/each}
    </nav>

    <div class="theme-switcher">
      <button on:click={() => theme.toggle()}>Тема: {$theme}</button>
    </div>
  </aside>

  <main class="content">
    <header class="topbar">
      <div>
        <h1>{shellState?.chrome.companyName ?? "Acta"}</h1>
        <p>{shellState?.chrome.userRole ?? "Завантаження shell..."}</p>
      </div>
      <div class="topbar-actions">
        <select value={shellState?.activeCompanyId} on:change={onCompanyChange}>
          {#each shellState?.companyItems ?? [] as company}
            <option value={company.id}>{company.name}</option>
          {/each}
        </select>
        <button on:click={() => palette.toggle()}>Ctrl+K</button>
      </div>
    </header>

    {#if currentScreen === "dashboard"}
      <Dashboard />
    {:else if currentScreen === "documents"}
      <Documents />
    {:else if currentScreen === "counterparties"}
      <Counterparties />
    {:else if currentScreen === "payments"}
      <Payments />
    {:else if currentScreen === "reports"}
      <Reports />
    {:else if currentScreen === "tasks"}
      <Tasks />
    {:else if currentScreen === "settings"}
      <Settings />
    {/if}
  </main>

  {#if $palette.open}
    <button type="button" class="palette-backdrop" aria-label="Закрити палітру команд" on:click={() => palette.close()}></button>
    <section class="palette">
      <input bind:this={paletteInput} placeholder="Пошук команд, екранів і документів" on:input={onPaletteInput} />
      <div class="palette-items">
        {#each $palette.items as item}
          <button
            class="palette-item"
            on:click={async () => {
              await palette.activate(item.payload);
              palette.close();
            }}
          >
            <div>
              <strong>{item.title}</strong>
              <p>{item.subtitle}</p>
            </div>
            <span>{item.shortcut}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}
</div>
```

- [ ] **Step 2: Перевірити TypeScript**

```bash
cd frontend && npm run check
```

Очікується: без помилок.

- [ ] **Step 3: Перевірити build**

```bash
cd frontend && npm run build
```

Очікується: `dist/` згенеровано без помилок.

- [ ] **Step 4: Запустити frontend-тести**

```bash
cd frontend && npm run test:frontend
```

Очікується: всі тести проходять (тести stores не залежать від App.svelte).

- [ ] **Step 5: Перевірити що App.svelte став меншим**

```bash
wc -l frontend/src/App.svelte
```

Очікується: ~120 рядків (було 1386).

- [ ] **Step 6: Commit**

```bash
git add frontend/src/App.svelte
git commit -m "refactor(frontend): extract screens into frontend/src/screens/, slim App.svelte to shell"
```
