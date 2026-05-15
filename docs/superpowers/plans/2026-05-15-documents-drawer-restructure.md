# Documents Drawer Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorder the Documents drawer form so company, direction, number/date, counterparty, items, and notes appear in the approved visual order.

**Architecture:** This is a frontend-only change in the Svelte screen and document-specific CSS. The drawer continues to use the existing `documentsStore`, `counterpartiesStore`, and editor callbacks; the only new dependency is `shellStore` for the active company name. Tests lock the DOM contract before implementation so the restructure does not drift.

**Tech Stack:** Svelte 4, TypeScript, Vitest/jsdom, Tauri invoke stores, existing Acta CSS tokens.

---

## File Structure

- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`
  - Add a mocked `shellStore`.
  - Add assertions for company display, counterparty placement, header cleanup, and new-doc field order.
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`
  - Import and bind `shellStore`.
  - Remove counterparty text from the drawer header.
  - Reorder the editor form and move notes after the items card.
- Modify: `frontend/src/styles/documents.css`
  - Add reusable read-only field styles.
  - Add notes textarea styles for the textarea moved outside `.editor-grid`.

No Rust, SQL, backend DTO, migration, or PDF changes are in scope.

---

### Task 1: Lock Drawer Form Contract With Tests

**Files:**
- Modify: `frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts`

- [ ] **Step 1: Add `shellStore` test state and mock**

In `vi.hoisted`, add `shellState` after `counterpartiesState`:

```ts
  const shellState = createMockStore({
    state: {
      chrome: {
        companyName: "ТОВ Акт",
        userName: "Олена Бухгалтер",
        userInitials: "ОБ",
        userRole: "Бухгалтер",
        documentsBadge: 2,
        tasksBadge: 1
      },
      companyItems: [],
      activeCompanyId: "company-1",
      isDark: false
    },
    loading: false,
    error: null,
    phase: "idle",
    pendingCompanyId: null,
    progressLabel: null
  });
```

Return it from the hoisted object:

```ts
  return {
    counterpartiesState,
    documentsState,
    shellState,
    addItem: vi.fn(),
```

Add this mock after the existing `counterparties` store mock:

```ts
vi.mock("../../stores/shell", () => ({
  shellStore: {
    subscribe: mocks.shellState.subscribe
  }
}));
```

- [ ] **Step 2: Add a helper for reading editor field order**

Add this helper after `buttonByText`:

```ts
function editorGridText(target: HTMLElement): string {
  const grid = target.querySelector(".editor-grid");
  expect(grid).toBeTruthy();
  return grid?.textContent ?? "";
}
```

- [ ] **Step 3: Write the failing existing-document layout test**

Add this test near the existing drawer/editor tests, after `"uses canonical button hierarchy in toolbar and editor header"`:

```ts
  it("renders company and existing counterparty inside the drawer form, not the header", () => {
    const { component, target } = renderDocuments();

    const header = target.querySelector(".documents-drawer > .editor-header");
    const grid = target.querySelector(".documents-drawer .editor-grid");
    const readonlyFields = Array.from(target.querySelectorAll(".documents-drawer .editor-field-readonly"));

    expect(header).toBeTruthy();
    expect(grid).toBeTruthy();
    expect(header?.querySelector("p")?.textContent).not.toContain("ТОВ Ромашка");
    expect(grid?.textContent).toContain("Компанія");
    expect(grid?.textContent).toContain("ТОВ Акт");
    expect(grid?.textContent).toContain("Контрагент");
    expect(grid?.textContent).toContain("ТОВ Ромашка");
    expect(readonlyFields.length).toBeGreaterThanOrEqual(2);

    component.$destroy();
  });
```

- [ ] **Step 4: Write the failing new-document field order test**

Add this test immediately after the existing-document layout test:

```ts
  it("keeps the new document counterparty select after direction and number/date", () => {
    mocks.documentsState.set({
      list: makeList(),
      editor: makeEditor(),
      chain: makeChain(),
      draftContext: null,
      pendingNew: true,
      selectedIds: [],
      initialLoading: false,
      loading: false,
      error: null,
      message: "Готово",
      activeTab: "all",
      kindFilter: null,
      counterpartyFilterId: null,
      dateFrom: null,
      dateTo: null,
      statusFilter: [],
      amountMin: null,
      amountMax: null,
      overdueOnly: false,
      activePresetId: null
    });
    const { component, target } = renderDocuments();

    const text = editorGridText(target);
    const companyIndex = text.indexOf("Компанія");
    const directionIndex = text.indexOf("Напрямок");
    const numberIndex = text.indexOf("Номер");
    const dateIndex = text.indexOf("Дата");
    const counterpartyIndex = text.indexOf("Контрагент");

    expect(companyIndex).toBeGreaterThanOrEqual(0);
    expect(directionIndex).toBeGreaterThan(companyIndex);
    expect(numberIndex).toBeGreaterThan(directionIndex);
    expect(dateIndex).toBeGreaterThan(numberIndex);
    expect(counterpartyIndex).toBeGreaterThan(dateIndex);
    expect(target.querySelector(".documents-drawer .editor-grid select")?.textContent).toContain("Оберіть контрагента");

    component.$destroy();
  });
```

- [ ] **Step 5: Write the failing notes placement/style contract test**

Add this source/CSS contract test near the existing source/CSS assertions:

```ts
  it("moves editor notes outside the grid and keeps textarea styling in document CSS", () => {
    expect(source).toContain('class="editor-notes-field"');
    expect(source).toMatch(/<div class="editor-items-card">[\s\S]*<\/div>[\s\S]*<label class="editor-notes-field">/);
    expect(styles).toContain(".editor-notes-field textarea");
    expect(styles).toMatch(/\.editor-notes-field textarea\s*\{[\s\S]*min-height:\s*96px/);
    expect(styles).toMatch(/\.editor-notes-field textarea\s*\{[\s\S]*resize:\s*vertical/);
  });
```

- [ ] **Step 6: Run the focused test file and verify failure**

Run:

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
```

Expected: FAIL. The failures should mention missing `shellStore` import/rendering, missing `.editor-field-readonly`, header still containing the counterparty, and missing `.editor-notes-field`.

---

### Task 2: Reorder Documents Drawer Markup

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Step 1: Import and bind `shellStore`**

Update the imports and constants at the top of the file:

```ts
  import { documentsStore } from "../stores/documents";
  import { counterpartiesStore } from "../stores/counterparties";
  import { shellStore } from "../stores/shell";
```

```ts
  const documents = documentsStore;
  const counterparties = counterpartiesStore;
  const shell = shellStore;
```

- [ ] **Step 2: Remove the header counterparty paragraph**

In the drawer header, delete this line:

```svelte
        <p>{$documents.editor.form.counterpartyName}</p>
```

Do not change `editor-header-meta`, the title, or the action buttons.

- [ ] **Step 3: Replace the editor grid block**

Replace the current `<div class="editor-grid">...</div>` block with:

```svelte
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
```

- [ ] **Step 4: Move notes after the items card**

Insert this block after the closing `</div>` of the main `editor-items-card` and before the optional existing PDF card:

```svelte
    <label class="editor-notes-field">
      Примітки
      <textarea
        rows="3"
        value={$documents.editor.form.notes}
        on:input={onEditorNotesChange}
        disabled={$documents.loading}
      ></textarea>
    </label>
```

- [ ] **Step 5: Run the focused test file**

Run:

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
```

Expected: the new behavior tests may still fail only on missing CSS classes; existing interaction tests should still pass.

---

### Task 3: Add Drawer Field CSS and Verify

**Files:**
- Modify: `frontend/src/styles/documents.css`

- [ ] **Step 1: Add read-only field styles**

Add this block near the existing editor/drawer styles, after `.editor-header-meta, .editor-items-summary` or another nearby editor section:

```css
.editor-field-readonly {
  background: color-mix(in srgb, var(--acta-color-accent-soft) 38%, var(--acta-color-bg-elevated));
  border: 1px solid color-mix(in srgb, var(--acta-color-accent) 22%, var(--acta-color-border));
  border-radius: 8px;
  padding: 9px 12px;
  display: flex;
  align-items: center;
  gap: 10px;
}

.editor-field-readonly-label {
  font-size: 10px;
  font-weight: 700;
  color: var(--acta-color-accent-text);
  min-width: 76px;
}

.editor-field-readonly-value {
  font-weight: 600;
  color: var(--acta-color-text);
}

.editor-field-readonly-hint {
  font-size: 10px;
  color: var(--acta-color-text-faint);
  margin-left: auto;
}
```

- [ ] **Step 2: Add notes field styles**

Add this block near the read-only field styles:

```css
.editor-notes-field {
  display: grid;
  gap: 8px;
}

.editor-notes-field textarea {
  width: 100%;
  box-sizing: border-box;
  min-height: 96px;
  border: 1px solid var(--acta-color-border);
  border-radius: var(--acta-radius-lg);
  padding: 8px 12px;
  background: var(--acta-color-bg-elevated);
  color: inherit;
  resize: vertical;
  font: inherit;
}
```

- [ ] **Step 3: Run the focused frontend tests**

Run:

```bash
npm run test:frontend -- frontend/src/lib/screens/__tests__/DocumentsScreen.test.ts
```

Expected: PASS for `DocumentsScreen.test.ts`.

- [ ] **Step 4: Run TypeScript/Svelte checks**

Run:

```bash
npm run check
```

Expected: PASS with no Svelte or TypeScript errors. This specifically verifies `$shell.state?.chrome.companyName` is typed correctly.

- [ ] **Step 5: Run all frontend tests**

Run:

```bash
npm run test:frontend
```

Expected: PASS.

---

### Task 4: Manual Visual QA

**Files:**
- No source changes expected unless visual QA reveals a regression.

- [ ] **Step 1: Start the frontend dev server**

Run:

```bash
npm run dev
```

Expected: Vite serves the frontend, normally at `http://localhost:1420/`.

- [ ] **Step 2: Open Documents and inspect an existing document drawer**

Open the app in the browser test harness or Tauri dev flow and navigate to Documents.

Expected visual order in the drawer:

```text
Header: kind/status/title/actions
Grid: Компанія
Grid: Напрямок
Grid: Номер + Дата
Grid: Контрагент
Позиції документа
Примітки
Existing PDF, if present
```

- [ ] **Step 3: Inspect new document drawer**

Create a new document draft.

Expected:

```text
Компанія is read-only.
Контрагент is a select.
Контрагент appears after Напрямок and Номер/Дата.
Примітки appears after Позиції документа.
```

- [ ] **Step 4: Check responsive layout**

Check desktop width around `1440px` and narrow width around `390px`.

Expected:

```text
Read-only fields do not overflow.
The "тільки перегляд" hint either fits or wraps cleanly.
Textarea keeps full width and normal form styling.
Drawer header still wraps actions cleanly.
```

---

## Self-Review

- Spec coverage:
  - Company field from `shellStore`: Task 1 and Task 2.
  - Counterparty moved from header to form: Task 1 and Task 2.
  - Direction before number/date: Task 1 and Task 2.
  - Notes after items and outside grid: Task 1 and Task 2.
  - Read-only styling and textarea styling: Task 1 and Task 3.
  - No backend/PDF/table/filter changes: preserved by file scope.
- Placeholder scan:
  - No `TBD`, `TODO`, or vague “add tests” instructions remain.
- Type consistency:
  - `shellStore` is read through `$shell.state?.chrome.companyName`, matching the current store shape.
  - Existing document state uses `$documents.editor.form.counterpartyName`, matching `DocumentDraftFormDto`.
