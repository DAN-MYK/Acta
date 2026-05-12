# Payments Bank/Calendar Tab Split — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `PaymentsScreen.svelte` into two horizontal tabs — "Банк" (bank reconciliation) and "Платіжний календар" (payment calendar) — each with its own KPI cards.

**Architecture:** Extract all bank content into a new `BankTabContent.svelte` component; `PaymentsScreen.svelte` becomes a thin shell with a tab switcher and the editor sheet. `PaymentCalendarPanel.svelte` gets KPI cards replacing its existing summary cards. Both sub-components read `paymentsStore` directly — no new props.

**Tech Stack:** Svelte 4, TypeScript, Vitest + jsdom. All tests in `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`. Run with: `npm run test:frontend` from `C:\Users\MykhailoDan\apps\Acta`.

---

## File Map

| File | Action |
|------|--------|
| `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts` | Add 3 tab-switching tests |
| `frontend/src/lib/components/BankTabContent.svelte` | Create — move bank content from PaymentsScreen |
| `frontend/src/lib/screens/PaymentsScreen.svelte` | Refactor — thin shell, add tab switcher |
| `frontend/src/lib/components/PaymentCalendarPanel.svelte` | Update — replace summary cards with KPI cards |

---

## Task 1: Write failing tab-switching tests

**Files:**
- Modify: `frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts`

- [ ] **Step 1.1: Add 3 tests at the end of the describe block (before the closing `})`)**

Open `PaymentsScreen.test.ts`. Find the last `it(...)` block (line ~1024–1050). Append these 3 tests inside the `describe("PaymentsScreen component", ...)` block, before its closing `}`:

```typescript
  it("shows bank tab content by default and hides calendar panel", () => {
    const { component, target } = renderPayments();

    expect(target.querySelector('[data-testid="payments-unmatched-group"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="payments-calendar"]')).toBeNull();

    component.$destroy();
  });

  it("switches to calendar tab on click and hides bank content", async () => {
    const { component, target } = renderPayments();

    buttonByText(target, "Платіжний календар").click();
    await tick();

    expect(target.querySelector('[data-testid="payments-unmatched-group"]')).toBeNull();
    expect(target.querySelector('[data-testid="payments-calendar"]')).toBeTruthy();

    component.$destroy();
  });

  it("switches back to bank tab from calendar tab", async () => {
    const { component, target } = renderPayments();

    buttonByText(target, "Платіжний календар").click();
    await tick();
    buttonByText(target, "Банк").click();
    await tick();

    expect(target.querySelector('[data-testid="payments-unmatched-group"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="payments-calendar"]')).toBeNull();

    component.$destroy();
  });
```

- [ ] **Step 1.2: Run tests to confirm the 3 new tests fail**

```
npm run test:frontend
```

Expected: 3 failures with messages like:
- `"shows bank tab content by default"` — fails because `data-testid="payments-calendar"` IS present (calendar renders unconditionally now)
- `"switches to calendar tab"` — fails because no button "Платіжний календар" exists as a tab
- `"switches back to bank tab"` — fails for same reason

All existing tests must still pass. If any pre-existing test fails, investigate before continuing.

---

## Task 2: Create BankTabContent.svelte

**Files:**
- Create: `frontend/src/lib/components/BankTabContent.svelte`

This component receives the entire bank-related content currently in `PaymentsScreen.svelte`. It imports `paymentsStore` directly — no props.

- [ ] **Step 2.1: Create the file with the script block**

Create `frontend/src/lib/components/BankTabContent.svelte` with this `<script>` block (copy from PaymentsScreen, removing editor-only items):

```svelte
<script lang="ts">
  import SkeletonCard from "./SkeletonCard.svelte";
  import SkeletonRow from "./SkeletonRow.svelte";
  import {
    PAYMENT_SCREEN_COPY,
    PAYMENT_FLOW_COPY,
    PAYMENT_MANUAL_PICKER_DISABLED_REASON
  } from "../config/ui";
  import { isFormattedMoneyNegative } from "../money";
  import {
    getPaymentCandidateHint,
    getPaymentDirectionLabel,
    getPaymentDocumentKindLabel,
    getPaymentPreviewCopy,
    getPaymentStateLabel
  } from "../paymentsPresentation";
  import { paymentsStore } from "../stores/payments";
  import type { PaymentDraftFormDto } from "../types";

  const payments = paymentsStore;
  let importButton: HTMLButtonElement | null = null;

  function onPaymentFieldChange(field: keyof PaymentDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
    payments.updateFormField(field, input.value);
  }

  function onManualSearchInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    payments.updateManualMatchQuery(input.value);
  }

  function onSplitAllocationInput(documentId: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    payments.updateSplitAllocationAmount(documentId, input.value);
  }

  function focusImportButton() {
    importButton?.focus();
  }

  function runHeaderReconciliation() {
    const target = unmatchedPayments[0];
    if (target) {
      payments.reconcile(target.id);
    }
  }

  function isPaymentBusy(paymentId: string): boolean {
    return $payments.loading && $payments.activePaymentId === paymentId;
  }

  function openManualPickerForCurrentPreview() {
    const paymentId = $payments.matchPreview?.paymentId;
    if (paymentId) {
      payments.openManualMatchPicker(paymentId);
    }
  }

  function getFlowCopy(): { title: string; description: string } | null {
    if (!$payments.loading || !$payments.activeAction) {
      return null;
    }
    return PAYMENT_FLOW_COPY[$payments.activeAction] ?? null;
  }

  $: items = $payments.list?.items ?? [];
  $: unmatchedPayments = items.filter((item) => !item.matchedDoc);
  $: matchedPayments = items.filter((item) => Boolean(item.matchedDoc));
  $: busyImport = $payments.loading && $payments.activeAction === "import";
  $: busyImportPick = $payments.loading && $payments.activeAction === "import-pick";
  $: busyImportCommit = $payments.loading && $payments.activeAction === "import-commit";
  $: busySync = $payments.loading && $payments.activeAction === "sync";
  $: busySave = $payments.loading && $payments.activeAction === "save";
  $: manualPickerCanConfirm = Boolean(
    $payments.manualPicker?.selectedCandidateId && ($payments.manualPicker?.candidates.length ?? 0) > 0
  );
  $: manualPickerDisabledReason =
    !$payments.manualPicker || $payments.manualPicker.candidates.length > 0
      ? ""
      : PAYMENT_MANUAL_PICKER_DISABLED_REASON;
  $: flowCopy = getFlowCopy();
  $: flowTitle = flowCopy?.title ?? null;
  $: flowDescription = flowCopy?.description ?? null;
  $: previewCopy = getPaymentPreviewCopy($payments.matchPreview);
</script>
```

- [ ] **Step 2.2: Add the template block**

Append the template to the file. This is the bank content from `PaymentsScreen.svelte` — copy lines 145–648 of that file **excluding line 202** (`<PaymentCalendarPanel />`). The root is a plain `<div class="bank-tab-root">` wrapper:

```svelte
<div class="bank-tab-root">
  <div class="payments-toolbar">
    <button
      bind:this={importButton}
      class="btn-primary"
      on:click={() => payments.pickAndPreviewImport()}
      disabled={busyImportPick || busyImport || busyImportCommit}
    >
      {busyImportPick ? PAYMENT_SCREEN_COPY.prepareImportPreview : PAYMENT_SCREEN_COPY.importStatement}
    </button>
    <button class="btn-secondary" on:click={() => payments.openEditor()} disabled={$payments.loading}>
      Створити платіж
    </button>
    <button
      class="btn-secondary"
      on:click={runHeaderReconciliation}
      disabled={unmatchedPayments.length === 0 || $payments.loading}
    >
      Запустити звірку{unmatchedPayments.length > 0 ? ` (${unmatchedPayments.length})` : ""}
    </button>
    <button
      class="btn-ghost"
      on:click={() => payments.importCsv()}
      disabled={busyImport || busyImportPick || busyImportCommit}
    >
      {busyImport ? PAYMENT_SCREEN_COPY.importing : PAYMENT_SCREEN_COPY.importFromStorage}
    </button>
    <button class="btn-ghost" on:click={() => payments.syncBank()} disabled={busyImport || busySync || busyImportPick}>
      {busySync ? PAYMENT_SCREEN_COPY.syncing : PAYMENT_SCREEN_COPY.syncWithBank}
    </button>
    <button class="btn-ghost" on:click={() => payments.openManualTemplate()} disabled={busyImport || busySync}>
      Шаблон CSV
    </button>
  </div>

  <div class="task-kpis" data-testid="payments-kpis">
    {#if $payments.initialLoading}
      <SkeletonCard count={4} />
    {:else}
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
        <span>Баланс</span>
      </div>
      <div class="task-kpi-card task-kpi-card-alert">
        <strong>{$payments.list?.kpi.unmatchedCount ?? 0}</strong>
        <span>Не зведено</span>
      </div>
    {/if}
  </div>

  <!-- Copy lines 204–648 from PaymentsScreen.svelte exactly as-is (flow-banner, status banners,
       importPreview section, matchPreview section with manualPicker and splitDraft, payments-groups).
       Do NOT copy line 202 (<PaymentCalendarPanel />) — it does not belong here. -->
</div>
```

> **Important:** After the `task-kpis` div, copy the remaining template from `PaymentsScreen.svelte` lines 204–648 verbatim (flow-banner, status banners, importPreview chain-panel, matchPreview chain-panel with manual picker / split draft, and the payments-groups section). Replace the comment above with that content.

- [ ] **Step 2.3: Add the style block**

Append the `<style>` block. Copy **all scoped CSS** from `PaymentsScreen.svelte` lines 763–1036 **except** these editor-only rules (which stay in PaymentsScreen):
- `.editor-dirty-banner`, `.editor-dirty-actions`
- `.payment-editor-grid`
- `.editor-grid`, `.editor-grid label`, `.editor-grid-span`

Keep everything else, including `.chain-panel`, `.chain-panel-header`, `.editor-actions`, `.editor-items-empty`, `.editor-header` (used in the reconciliation preview panels), `.task-kpis`, `.task-kpi-card`, `.task-kpi-card-alert`, `.payments-toolbar`, `.payments-group`, `.payment-row`, `.payment-state`, `.payment-import-preview-*`, etc.

Also keep the `@media (max-width: 1080px)` block but remove `.editor-grid` from the grid-template reset inside it.

---

## Task 3: Refactor PaymentsScreen.svelte into a thin shell

**Files:**
- Modify: `frontend/src/lib/screens/PaymentsScreen.svelte`

Replace the **entire file** with the following content. This keeps: panel header, tab switcher, conditional rendering of BankTabContent or PaymentCalendarPanel, and the editor sheet/backdrop.

- [ ] **Step 3.1: Replace PaymentsScreen.svelte**

```svelte
<script lang="ts">
  import PaymentCalendarPanel from "../components/PaymentCalendarPanel.svelte";
  import BankTabContent from "../components/BankTabContent.svelte";
  import { EDITOR_DIRTY_COPY } from "../config/ui";
  import { paymentsStore } from "../stores/payments";

  const payments = paymentsStore;
  let activeTab: "bank" | "calendar" = "bank";
  let pendingDirtyClose = false;

  $: busySave = $payments.loading && $payments.activeAction === "save";

  function closeEditor(force = false) {
    const result = payments.closeEditor(force);
    if (result && result.ok === false && result.reason === "dirty") {
      pendingDirtyClose = true;
      return result;
    }
    pendingDirtyClose = false;
    return result;
  }

  function requestCloseEditor() {
    closeEditor();
  }

  function confirmDiscardChanges() {
    closeEditor(true);
  }

  function cancelDiscardChanges() {
    pendingDirtyClose = false;
  }

  function onEditorBackdropClick() {
    requestCloseEditor();
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if ($payments.editor && event.key === "Escape") {
      requestCloseEditor();
    }
  }

  $: if (!$payments.editor && pendingDirtyClose) {
    pendingDirtyClose = false;
  }
</script>

<svelte:window on:keydown={onWindowKeydown} />

<section
  class="panel"
  data-testid="payments-screen"
  inert={$payments.editor ? true : undefined}
  aria-hidden={$payments.editor ? "true" : undefined}
>
  <div class="panel-header">
    <div>
      <h2>Платежі</h2>
      <p>{$payments.list?.items.length ?? 0} записів</p>
    </div>
  </div>

  <div class="payments-tabs" role="tablist">
    <button
      class="payments-tab"
      class:active={activeTab === "bank"}
      role="tab"
      aria-selected={activeTab === "bank"}
      on:click={() => (activeTab = "bank")}
    >Банк</button>
    <button
      class="payments-tab"
      class:active={activeTab === "calendar"}
      role="tab"
      aria-selected={activeTab === "calendar"}
      on:click={() => (activeTab = "calendar")}
    >Платіжний календар</button>
  </div>

  {#if activeTab === "bank"}
    <BankTabContent />
  {:else}
    <PaymentCalendarPanel />
  {/if}
</section>

{#if $payments.editor}
  <button
    type="button"
    class="editor-backdrop"
    aria-label="Закрити редактор"
    data-testid="payments-editor-backdrop"
    on:click={onEditorBackdropClick}
  ></button>
  <section class="editor-sheet" role="dialog" aria-modal="true">
    {#if pendingDirtyClose}
      <div
        class="editor-dirty-banner"
        role="alertdialog"
        aria-live="assertive"
        aria-labelledby="payments-dirty-banner-title"
        data-testid="payments-dirty-banner"
      >
        <div>
          <strong id="payments-dirty-banner-title">{EDITOR_DIRTY_COPY.dirtyTitle}</strong>
          <p>{EDITOR_DIRTY_COPY.dirtyDescription}</p>
        </div>
        <div class="editor-dirty-actions">
          <button
            type="button"
            class="btn-ghost btn-sm"
            on:click={cancelDiscardChanges}
            data-testid="payments-dirty-banner-cancel"
          >{EDITOR_DIRTY_COPY.dirtyStay}</button>
          <button
            type="button"
            class="btn-danger btn-sm"
            on:click={confirmDiscardChanges}
            data-testid="payments-dirty-banner-discard"
          >{EDITOR_DIRTY_COPY.dirtyDiscard}</button>
        </div>
      </div>
    {/if}
    <div class="editor-header">
      <div>
        <h3>{$payments.editor.id ? "Редагувати платіж" : "Новий платіж"}</h3>
        <p>Картка платежу</p>
      </div>
      <div class="editor-actions">
        <button class="btn-primary" on:click={() => payments.save()} disabled={busySave}>Зберегти</button>
        <button class="btn-ghost" on:click={requestCloseEditor} disabled={busySave}>Закрити</button>
      </div>
    </div>

    <div class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>Що перевірити перед збереженням</strong>
          <p>Перевірте напрям, суму, контрагента та референс, щоб звірка не губилася після імпорту.</p>
        </div>
        <div class="chain-summary">
          <div class="chain-summary-block">
            <span>Напрям</span>
            <strong>{$payments.editor.direction === "income" ? "Надходження" : "Витрата"}</strong>
          </div>
          <div class="chain-summary-block">
            <span>Пов'язаний документ</span>
            <strong>{$payments.editor.description || "Ще не вказано"}</strong>
          </div>
        </div>
      </div>
    </div>

    <div class="editor-grid payment-editor-grid">
      <label>
        Дата
        <input
          type="date"
          value={$payments.editor.date}
          on:input={(event) => payments.updateFormField("date", (event.currentTarget as HTMLInputElement).value)}
        />
      </label>
      <label>
        Напрям
        <select
          value={$payments.editor.direction}
          on:change={(event) => payments.updateFormField("direction", (event.currentTarget as HTMLSelectElement).value)}
        >
          <option value="income">Надходження</option>
          <option value="expense">Витрата</option>
        </select>
      </label>
      <label>
        Сума
        <input
          value={$payments.editor.amount}
          on:input={(event) => payments.updateFormField("amount", (event.currentTarget as HTMLInputElement).value)}
        />
      </label>
      <label>
        Контрагент
        <select
          value={$payments.editor.counterpartyId}
          on:change={(event) => payments.updateFormField("counterpartyId", (event.currentTarget as HTMLSelectElement).value)}
        >
          <option value="">- Без контрагента -</option>
          {#each $payments.list?.counterparties ?? [] as cp}
            <option value={cp.id}>{cp.name}</option>
          {/each}
        </select>
      </label>
      <label>
        Референс платежу
        <input
          value={$payments.editor.reference}
          on:input={(event) => payments.updateFormField("reference", (event.currentTarget as HTMLInputElement).value)}
        />
      </label>
      <label>
        Пов'язаний документ
        <input
          value={$payments.editor.description}
          on:input={(event) => payments.updateFormField("description", (event.currentTarget as HTMLInputElement).value)}
        />
      </label>
      <label class="editor-grid-span">
        Банк
        <input
          value={$payments.editor.bankName}
          on:input={(event) => payments.updateFormField("bankName", (event.currentTarget as HTMLInputElement).value)}
        />
      </label>
    </div>
  </section>
{/if}

<style>
  .payments-tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--acta-color-border);
    margin-top: 18px;
  }

  .payments-tab {
    padding: 10px 20px;
    border: 0;
    background: transparent;
    color: var(--acta-color-text-muted);
    font-weight: 500;
    cursor: pointer;
    position: relative;
  }

  .payments-tab:hover {
    color: var(--acta-color-text);
  }

  .payments-tab.active {
    color: var(--acta-color-accent-text);
  }

  .payments-tab.active::after {
    content: "";
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--acta-color-accent);
    border-radius: 2px 2px 0 0;
  }

  .editor-sheet {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-elevated);
    display: grid;
    gap: 16px;
  }

  .editor-header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  .editor-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }

  .editor-dirty-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    border-radius: var(--acta-radius-xl);
    background: color-mix(in srgb, var(--acta-color-danger-soft) 40%, var(--acta-color-bg-elevated));
    border: 1px solid color-mix(in srgb, var(--acta-color-danger) 22%, var(--acta-color-border));
  }

  .editor-dirty-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .chain-panel {
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--acta-color-accent-soft) 32%, transparent), transparent 74%),
      var(--acta-color-bg-elevated);
  }

  .chain-panel-header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  .chain-panel-header p {
    margin: 6px 0 0;
    color: var(--acta-color-text-muted);
  }

  .chain-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .chain-summary-block {
    display: grid;
    gap: 6px;
    padding: 14px 16px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 76%, var(--acta-color-bg-elevated));
  }

  .chain-summary-block span {
    font-size: 12px;
    color: var(--acta-color-text-muted);
  }

  .editor-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }

  .editor-grid label {
    display: grid;
    gap: 8px;
  }

  .editor-grid-span {
    grid-column: 1 / -1;
  }

  @media (max-width: 1080px) {
    .editor-grid,
    .chain-summary {
      grid-template-columns: 1fr;
    }

    .editor-header {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
```

- [ ] **Step 3.2: Run tests — all must pass including the 3 new ones**

```
npm run test:frontend
```

Expected: All tests pass. If the 3 new tab tests still fail:
- "shows bank tab content by default" — verify `BankTabContent` renders `data-testid="payments-unmatched-group"`
- "switches to calendar tab" — verify tab button text is exactly "Платіжний календар"
- Any test about `getPaymentDirectionLabel` in editor — that function is no longer called in PaymentsScreen; the direction label in the editor uses a ternary inline. Verify the editor test `uses canonical date control` still passes.

- [ ] **Step 3.3: Commit**

```
git add frontend/src/lib/components/BankTabContent.svelte frontend/src/lib/screens/PaymentsScreen.svelte frontend/src/lib/screens/__tests__/PaymentsScreen.test.ts
git commit -m "feat: split payments into Bank and Calendar tabs"
```

---

## Task 4: Update PaymentCalendarPanel KPI cards

**Files:**
- Modify: `frontend/src/lib/components/PaymentCalendarPanel.svelte`

Replace the `<div class="calendar-summary">` block (lines 142–155) and its CSS (`.calendar-summary`, `.calendar-summary-card`) with `task-kpi-card`-styled cards.

- [ ] **Step 4.1: Replace the calendar-summary block in the template**

Find this block in `PaymentCalendarPanel.svelte` (around line 142):

```svelte
  <div class="calendar-summary">
    <div class="calendar-summary-card">
      <strong>{scheduleCount}</strong>
      <span>Планових платежів у місяці</span>
    </div>
    <div class="calendar-summary-card">
      <strong>{taskCount}</strong>
      <span>Дедлайнів задач у місяці</span>
    </div>
    <div class="calendar-summary-card">
      <strong>{visibleEventCount}</strong>
      <span>{PAYMENT_CALENDAR_COPY.visibleEventsSummary}</span>
    </div>
  </div>
```

Replace it with:

```svelte
  <div class="task-kpis">
    <div class="task-kpi-card">
      <strong>{scheduleCount}</strong>
      <span>Планових платежів у місяці</span>
    </div>
    <div class="task-kpi-card">
      <strong>{taskCount}</strong>
      <span>Дедлайнів задач у місяці</span>
    </div>
    <div class="task-kpi-card">
      <strong>{visibleEventCount}</strong>
      <span>{PAYMENT_CALENDAR_COPY.visibleEventsSummary}</span>
    </div>
  </div>
```

- [ ] **Step 4.2: Replace calendar-summary CSS with task-kpi-card CSS**

In `PaymentCalendarPanel.svelte`'s `<style>` block, find and remove:

```css
  .calendar-summary {
    flex-wrap: wrap;
  }

  .calendar-summary-card,
  .calendar-grid-panel,
  .calendar-side-panel {
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-elevated) 82%, white 18%);
  }

  .calendar-summary-card {
    min-width: 180px;
    padding: 14px 16px;
    display: grid;
    gap: 6px;
  }

  .calendar-summary-card strong {
    font-size: 1.35rem;
  }

  .calendar-summary-card span {
    color: var(--acta-color-text-muted);
  }
```

Replace with (keep the `.calendar-grid-panel` and `.calendar-side-panel` border rules separately, add task-kpi-card):

```css
  .calendar-grid-panel,
  .calendar-side-panel {
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-elevated) 82%, white 18%);
  }

  .task-kpis {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 16px;
  }

  .task-kpi-card {
    display: grid;
    gap: 10px;
    padding: 16px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-subtle) 72%, var(--acta-color-bg-elevated));
  }

  .task-kpi-card strong {
    font-size: 1.35rem;
  }

  .task-kpi-card span {
    font-size: 12px;
    color: var(--acta-color-text-muted);
  }

  @media (max-width: 960px) {
    .task-kpis {
      grid-template-columns: 1fr;
    }
  }
```

> Note: The `@media (max-width: 960px)` rule for `.task-kpis` should be placed inside the existing `@media (max-width: 960px)` block already present in the file, not as a separate block.

- [ ] **Step 4.3: Run tests — all must still pass**

```
npm run test:frontend
```

Expected: All tests pass. Calendar KPI change has no test coverage (calendar state is `null` in most tests), so no breakage expected.

- [ ] **Step 4.4: Commit**

```
git add frontend/src/lib/components/PaymentCalendarPanel.svelte
git commit -m "feat: replace calendar summary cards with task-kpi-card style"
```

---

## Task 5: Verify full test suite

- [ ] **Step 5.1: Run all tests one final time**

```
npm run test:frontend
```

Expected output:
```
Test Files  X passed
Tests       X passed (all)
```

All pre-existing tests pass. The 3 new tab-switching tests pass. No regressions.

- [ ] **Step 5.2: Run Svelte type-check**

```
npm run check
```

Expected: No type errors. If `getPaymentDirectionLabel` is reported as unused in BankTabContent, verify it is actually called in the template (direction label in payment row). If unused, remove the import.
