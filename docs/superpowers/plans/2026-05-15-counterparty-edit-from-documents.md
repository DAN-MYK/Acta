# Counterparty Edit/Create from Documents Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users edit counterparty details and create new counterparties from within the document drawer, without leaving the Documents screen.

**Architecture:** A new presentational `CounterpartyModal.svelte` wraps the existing `Modal.svelte` and renders the counterparty form. All state lives in `documentsStore` as a `cpModal` slice; the modal never touches stores or API directly. A new Rust function `document_change_counterparty` updates `acts`/`invoices`/`waybills` tables using the existing `parse_document_ref` helper, exposed as a Tauri command.

**Tech Stack:** Svelte 4 + TypeScript, Rust + sqlx (runtime-style queries), Tauri 2 commands, Vitest + jsdom component tests.

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `frontend/src/lib/types.ts` | Modify | Add `ChangeCounterpartyResultDto` interface |
| `frontend/src/lib/api.ts` | Modify | Add `documentChangeCounterparty()` wrapper |
| `frontend/src/lib/browser-fixtures.ts` | Modify | Fix `counterparty_open_editor` branching; add `document_change_counterparty` mock |
| `frontend/src/lib/components/CounterpartyModal.svelte` | **Create** | Presentational modal — form + dirty confirm |
| `frontend/src/lib/components/__tests__/CounterpartyModal.test.ts` | **Create** | Component tests |
| `frontend/src/lib/stores/documents.ts` | Modify | Add `cpModal` state slice + 7 functions |
| `frontend/src/lib/stores/__tests__/documents.test.ts` | Modify | Add cpModal + changeCounterparty tests |
| `frontend/src/lib/screens/DocumentsScreen.svelte` | Modify | Counterparty section UI + modal mount |
| `src/tauri_api/documents/api.rs` | Modify | Add `ChangeCounterpartyResultDto` struct + business logic fn |
| `src-tauri/src/commands/documents.rs` | Modify | Add `document_change_counterparty` Tauri command |
| `src-tauri/src/lib.rs` | Modify | Register command in `invoke_handler` |

---

## Task 1: Add ChangeCounterpartyResultDto and API wrapper

**Files:**
- Modify: `frontend/src/lib/types.ts:278`
- Modify: `frontend/src/lib/api.ts:52,148`

- [ ] **Step 1: Add ChangeCounterpartyResultDto to types.ts**

In `frontend/src/lib/types.ts`, insert after line 278 (after the closing `}` of `CreateDocumentContextDto`):

```typescript
export interface ChangeCounterpartyResultDto {
  ok: boolean;
  counterpartyId: string;
  counterpartyName: string;
}
```

- [ ] **Step 2: Add import in api.ts**

In `frontend/src/lib/api.ts`, add `ChangeCounterpartyResultDto` to the existing type import block (lines 3–52). The import block ends with `TasksScreenDto` at line 51. Add after `TaskSaveResultDto`:

```typescript
  ChangeCounterpartyResultDto,
```

- [ ] **Step 3: Add documentChangeCounterparty function in api.ts**

In `frontend/src/lib/api.ts`, insert after line 148 (after `export function documentDelete(...)`):

```typescript
export function documentChangeCounterparty(
  docId: string,
  counterpartyId: string,
): Promise<ChangeCounterpartyResultDto> {
  return appInvoke("document_change_counterparty", { docId, counterpartyId });
}
```

- [ ] **Step 4: Verify TypeScript**

```bash
cd frontend && npm run check 2>&1 | tail -5
```
Expected: `0 errors` (or same count as before this task).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/types.ts frontend/src/lib/api.ts
git commit -m "feat(documents): add ChangeCounterpartyResultDto and documentChangeCounterparty API wrapper"
```

---

## Task 2: Update browser fixtures

**Files:**
- Modify: `frontend/src/lib/browser-fixtures.ts:1,604`

- [ ] **Step 1: Add ChangeCounterpartyResultDto to import in browser-fixtures.ts**

In `frontend/src/lib/browser-fixtures.ts`, add `ChangeCounterpartyResultDto` to the import block (lines 1–38). Insert after `CreateDocumentContextDto,`:

```typescript
  ChangeCounterpartyResultDto,
```

- [ ] **Step 2: Fix counterparty_open_editor to branch by payload**

In `frontend/src/lib/browser-fixtures.ts`, replace lines 604–619 (the entire `case "counterparty_open_editor":` block):

```typescript
    case "counterparty_open_editor": {
      const cpId = (payload as { counterpartyId?: string | null } | undefined)?.counterpartyId;
      if (!cpId) {
        return clone({
          form: {
            id: "",
            title: "Новий контрагент",
            name: "",
            edrpou: "",
            ipn: "",
            iban: "",
            address: "",
            phone: "",
            email: "",
            notes: "",
          },
          showEditor: true,
        } satisfies CounterpartyEditorDto) as T;
      }
      const found = counterparties.find((cp) => cp.id === cpId);
      return clone({
        form: {
          id: cpId,
          title: found?.name ?? "Контрагент",
          name: found?.name ?? "",
          edrpou: found?.edrpou ?? "",
          ipn: "",
          iban: "UA123456789012345678901234567",
          address: "м. Київ, вул. Хрещатик, 1",
          phone: "+380671112233",
          email: "office@example.com",
          notes: "",
        },
        showEditor: true,
      } satisfies CounterpartyEditorDto) as T;
    }
```

- [ ] **Step 3: Add document_change_counterparty mock**

In `frontend/src/lib/browser-fixtures.ts`, insert the new case immediately after the `case "counterparty_save":` block (after line 627):

```typescript
    case "document_change_counterparty": {
      const p = payload as { counterpartyId: string };
      const found = counterparties.find((cp) => cp.id === p.counterpartyId);
      return clone({
        ok: true,
        counterpartyId: p.counterpartyId,
        counterpartyName: found?.name ?? "Контрагент",
      } satisfies ChangeCounterpartyResultDto) as T;
    }
```

- [ ] **Step 4: Run frontend tests to ensure fixtures still work**

```bash
cd frontend && npm run test:frontend 2>&1 | tail -10
```
Expected: all tests pass, no new failures.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/browser-fixtures.ts
git commit -m "feat(documents): update fixtures — branch counterparty_open_editor by id, add document_change_counterparty mock"
```

---

## Task 3: CounterpartyModal component (TDD)

**Files:**
- Create: `frontend/src/lib/components/__tests__/CounterpartyModal.test.ts`
- Create: `frontend/src/lib/components/CounterpartyModal.svelte`

- [ ] **Step 1: Write failing tests**

Create `frontend/src/lib/components/__tests__/CounterpartyModal.test.ts`:

```typescript
/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import CounterpartyModal from "../CounterpartyModal.svelte";
import type { CounterpartyDraftFormDto } from "../../types";

const mockForm: CounterpartyDraftFormDto = {
  id: "cp-1",
  title: "ТОВ Ромашка",
  name: "ТОВ Ромашка",
  edrpou: "12345678",
  ipn: "",
  iban: "UA123",
  address: "",
  phone: "",
  email: "",
  notes: "",
};

function mount(props: Record<string, unknown>) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new CounterpartyModal({ target, props });
  return { component, target };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("CounterpartyModal", () => {
  it("renders 'Новий контрагент' title in create mode", () => {
    const { target, component } = mount({ isOpen: true, mode: "create", form: { ...mockForm, id: "" } });
    expect(target.querySelector("#modal-title")?.textContent).toBe("Новий контрагент");
    component.$destroy();
  });

  it("renders 'Редагування контрагента' title in edit mode", () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm });
    expect(target.querySelector("#modal-title")?.textContent).toBe("Редагування контрагента");
    component.$destroy();
  });

  it("shows dirty-confirm block when showCloseConfirm is true", () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm, showCloseConfirm: true });
    expect(target.querySelector("[data-testid='cp-modal-dirty-confirm']")).toBeTruthy();
    component.$destroy();
  });

  it("hides dirty-confirm block when showCloseConfirm is false", () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm, showCloseConfirm: false });
    expect(target.querySelector("[data-testid='cp-modal-dirty-confirm']")).toBeNull();
    component.$destroy();
  });

  it("dispatches closeConfirmed when 'Так, закрити' is clicked", async () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm, showCloseConfirm: true });
    const handler = vi.fn();
    component.$on("closeConfirmed", handler);
    const btn = Array.from(target.querySelectorAll("button")).find(
      (b) => b.textContent?.trim() === "Так, закрити"
    );
    btn?.click();
    await Promise.resolve();
    expect(handler).toHaveBeenCalledOnce();
    component.$destroy();
  });

  it("dispatches closeCancelled when 'Залишитись' is clicked", async () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm, showCloseConfirm: true });
    const handler = vi.fn();
    component.$on("closeCancelled", handler);
    const btn = Array.from(target.querySelectorAll("button")).find(
      (b) => b.textContent?.trim() === "Залишитись"
    );
    btn?.click();
    await Promise.resolve();
    expect(handler).toHaveBeenCalledOnce();
    component.$destroy();
  });

  it("dispatches save when 'Зберегти' is clicked", async () => {
    const { target, component } = mount({ isOpen: true, mode: "create", form: { ...mockForm, id: "" } });
    const handler = vi.fn();
    component.$on("save", handler);
    const btn = Array.from(target.querySelectorAll("button")).find(
      (b) => b.textContent?.trim() === "Зберегти"
    );
    btn?.click();
    await Promise.resolve();
    expect(handler).toHaveBeenCalledOnce();
    component.$destroy();
  });

  it("does not render when isOpen is false", () => {
    const { target, component } = mount({ isOpen: false, mode: "create", form: mockForm });
    expect(target.querySelector(".modal-backdrop")).toBeNull();
    component.$destroy();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd frontend && npx vitest run --config vitest.config.mjs -t "CounterpartyModal" 2>&1 | tail -15
```
Expected: `FAIL` — `CounterpartyModal.svelte` does not exist yet.

- [ ] **Step 3: Implement CounterpartyModal.svelte**

Create `frontend/src/lib/components/CounterpartyModal.svelte`:

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import Modal from "./Modal.svelte";
  import type { CounterpartyDraftFormDto } from "../types";

  export let isOpen: boolean;
  export let mode: "create" | "edit";
  export let form: CounterpartyDraftFormDto | null;
  export let loading: boolean = false;
  export let isDirty: boolean = false;
  export let showCloseConfirm: boolean = false;

  const dispatch = createEventDispatcher<{
    fieldChange: { field: keyof CounterpartyDraftFormDto; value: string };
    save: void;
    close: void;
    closeConfirmed: void;
    closeCancelled: void;
  }>();

  function onInput(field: keyof CounterpartyDraftFormDto, e: Event) {
    dispatch("fieldChange", { field, value: (e.target as HTMLInputElement | HTMLTextAreaElement).value });
  }
</script>

<Modal
  open={isOpen}
  title={mode === "create" ? "Новий контрагент" : "Редагування контрагента"}
  maxWidth={520}
  on:close={() => dispatch("close")}
>
  {#if form}
    {#if showCloseConfirm}
      <div class="cp-modal-dirty-confirm" data-testid="cp-modal-dirty-confirm">
        <p>Є незбережені зміни. Закрити без збереження?</p>
        <div class="cp-modal-dirty-actions">
          <button class="btn-danger" on:click={() => dispatch("closeConfirmed")}>Так, закрити</button>
          <button class="btn-ghost" on:click={() => dispatch("closeCancelled")}>Залишитись</button>
        </div>
      </div>
    {/if}

    <div class="cp-editor-grid">
      <label>
        Назва
        <input value={form.name} on:input={(e) => onInput("name", e)} disabled={loading} />
      </label>
      <label>
        ЄДРПОУ
        <input value={form.edrpou} on:input={(e) => onInput("edrpou", e)} disabled={loading} />
      </label>
      <label>
        ІПН
        <input value={form.ipn} on:input={(e) => onInput("ipn", e)} disabled={loading} />
      </label>
      <label>
        IBAN
        <input value={form.iban} on:input={(e) => onInput("iban", e)} disabled={loading} />
      </label>
      <label>
        Телефон
        <input value={form.phone} on:input={(e) => onInput("phone", e)} disabled={loading} />
      </label>
      <label>
        Email
        <input value={form.email} on:input={(e) => onInput("email", e)} disabled={loading} />
      </label>
      <label class="cp-editor-grid-span">
        Адреса
        <input value={form.address} on:input={(e) => onInput("address", e)} disabled={loading} />
      </label>
      <label class="cp-editor-grid-span">
        Примітки
        <textarea rows={4} value={form.notes} on:input={(e) => onInput("notes", e)} disabled={loading}></textarea>
      </label>
    </div>
  {/if}

  <svelte:fragment slot="footer">
    <button class="btn-primary" on:click={() => dispatch("save")} disabled={loading || !form}>
      Зберегти
    </button>
    <button class="btn-ghost" on:click={() => dispatch("close")} disabled={loading}>
      Скасувати
    </button>
  </svelte:fragment>
</Modal>

<style>
  .cp-editor-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--acta-space-3);
  }

  .cp-editor-grid-span {
    grid-column: span 2;
  }

  .cp-modal-dirty-confirm {
    background: color-mix(in srgb, var(--acta-color-warning, #f59e0b) 10%, transparent);
    border: 1px solid var(--acta-color-warning, #f59e0b);
    border-radius: var(--acta-radius-md);
    padding: var(--acta-space-3);
    margin-bottom: var(--acta-space-4);
  }

  .cp-modal-dirty-confirm p {
    margin: 0 0 var(--acta-space-2);
    font-size: 14px;
  }

  .cp-modal-dirty-actions {
    display: flex;
    gap: var(--acta-space-2);
  }
</style>
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd frontend && npx vitest run --config vitest.config.mjs -t "CounterpartyModal" 2>&1 | tail -15
```
Expected: `PASS` — all 7 tests pass.

- [ ] **Step 5: Run full frontend test suite**

```bash
cd frontend && npm run test:frontend 2>&1 | tail -5
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/components/CounterpartyModal.svelte \
        frontend/src/lib/components/__tests__/CounterpartyModal.test.ts
git commit -m "feat(documents): add CounterpartyModal component with dirty-confirm"
```

---

## Task 4: Documents store — cpModal state (TDD)

**Files:**
- Modify: `frontend/src/lib/stores/__tests__/documents.test.ts`
- Modify: `frontend/src/lib/stores/documents.ts`

- [ ] **Step 1: Write failing store tests**

In `frontend/src/lib/stores/__tests__/documents.test.ts`, add the following to the existing `vi.mock("../../api", ...)` mock object — add these three new mock functions alongside the existing ones:

```typescript
    counterpartyOpenEditor: vi.fn(),
    counterpartySave: vi.fn(),
    documentChangeCounterparty: vi.fn(),
```

Then add the following imports and test suite after the existing `describe` block:

```typescript
import type { CounterpartyEditorDto, CounterpartySaveResultDto } from "../../types";

const counterpartyOpenEditorMock = api.counterpartyOpenEditor as ReturnType<typeof vi.fn>;
const counterpartySaveMock = api.counterpartySave as ReturnType<typeof vi.fn>;
const documentChangeCounterpartyMock = api.documentChangeCounterparty as ReturnType<typeof vi.fn>;

const emptyEditorDto: CounterpartyEditorDto = {
  form: {
    id: "",
    title: "Новий контрагент",
    name: "",
    edrpou: "",
    ipn: "",
    iban: "",
    address: "",
    phone: "",
    email: "",
    notes: "",
  },
  showEditor: true,
};

const filledEditorDto: CounterpartyEditorDto = {
  form: {
    id: "cp-1",
    title: "ТОВ Ромашка",
    name: "ТОВ Ромашка",
    edrpou: "12345678",
    ipn: "",
    iban: "UA123",
    address: "",
    phone: "",
    email: "",
    notes: "",
  },
  showEditor: true,
};

function getState() {
  let s: any;
  documentsStore.subscribe((st) => { s = st; })();
  return s;
}

describe("documentsStore cpModal", () => {
  beforeEach(() => {
    counterpartyOpenEditorMock.mockReset();
    counterpartySaveMock.mockReset();
    documentChangeCounterpartyMock.mockReset();
  });

  it("openCpCreate sets cpModal with mode create and isOpen true", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    const state = getState();
    expect(state.cpModal).not.toBeNull();
    expect(state.cpModal.mode).toBe("create");
    expect(state.cpModal.isOpen).toBe(true);
    expect(state.cpModal.confirmClose).toBe(false);
    expect(state.cpModal.form).toEqual(emptyEditorDto.form);
    expect(state.cpModal.snapshot).toEqual(emptyEditorDto.form);
  });

  it("openCpEdit sets cpModal with mode edit and filled form", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(filledEditorDto);
    await documentsStore.openCpEdit("cp-1");
    const state = getState();
    expect(state.cpModal.mode).toBe("edit");
    expect(state.cpModal.form.name).toBe("ТОВ Ромашка");
    expect(counterpartyOpenEditorMock).toHaveBeenCalledWith("cp-1");
  });

  it("updateCpField updates form field without touching snapshot", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.updateCpField("name", "Нова назва");
    const state = getState();
    expect(state.cpModal.form.name).toBe("Нова назва");
    expect(state.cpModal.snapshot.name).toBe("");
  });

  it("closeCpModal when not dirty sets cpModal to null", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.closeCpModal();
    expect(getState().cpModal).toBeNull();
  });

  it("closeCpModal when dirty sets confirmClose to true without closing", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.updateCpField("name", "змінено");
    documentsStore.closeCpModal();
    const state = getState();
    expect(state.cpModal).not.toBeNull();
    expect(state.cpModal.confirmClose).toBe(true);
  });

  it("confirmCloseCpModal closes modal regardless of dirty state", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.updateCpField("name", "змінено");
    documentsStore.confirmCloseCpModal();
    expect(getState().cpModal).toBeNull();
  });

  it("cancelCloseCpModal sets confirmClose back to false", async () => {
    counterpartyOpenEditorMock.mockResolvedValue(emptyEditorDto);
    await documentsStore.openCpCreate();
    documentsStore.updateCpField("name", "змінено");
    documentsStore.closeCpModal();
    documentsStore.cancelCloseCpModal();
    const state = getState();
    expect(state.cpModal).not.toBeNull();
    expect(state.cpModal.confirmClose).toBe(false);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd frontend && npx vitest run --config vitest.config.mjs -t "documentsStore cpModal" 2>&1 | tail -15
```
Expected: `FAIL` — functions not found on `documentsStore`.

- [ ] **Step 3: Add cpModal state and imports to documents.ts**

In `frontend/src/lib/stores/documents.ts`, update the import from `"../api"` (lines 1–17) to add three new imports:

```typescript
import {
  counterpartyOpenEditor,
  counterpartySave,
  documentAdvanceStatus,
  documentChangeCounterparty,
  documentChainCreateDraft,
  documentChainGet,
  documentCreateDraft,
  documentDelete,
  documentGeneratePdf,
  documentOpen,
  documentPdfApplyTextReplace,
  documentPdfAttachExisting,
  documentPdfOpenCurrent,
  documentSave,
  documentsBulkAdvanceStatus,
  documentsBulkDelete,
  documentsList
} from "../api";
```

Add `CounterpartyDraftFormDto` to the types import (line 23):

```typescript
import type { CounterpartyDraftFormDto, DocumentChainDto, DocumentEditorDto, DocumentKind, DocumentsListDto } from "../types";
```

Add the `CpModalState` type and `cpModal` field to `DocumentsState`:

After line 26 (`type EditorPayload = ...`), insert:

```typescript
type CpModalState = {
  isOpen: boolean;
  mode: "create" | "edit";
  form: CounterpartyDraftFormDto;
  snapshot: CounterpartyDraftFormDto;
  loading: boolean;
  confirmClose: boolean;
};
```

In the `DocumentsState` interface (lines 32–54), add one new field before the closing `}`:

```typescript
  cpModal: CpModalState | null;
```

In `initialState` (lines 56–78), add:

```typescript
  cpModal: null,
```

- [ ] **Step 4: Add the 7 new store functions**

In `frontend/src/lib/stores/documents.ts`, add the following functions to the returned store object (after `clearDraftContext` / before `clearMessage` works well — around line 241). Add each function:

```typescript
    async openCpCreate(): Promise<void> {
      const editor = await counterpartyOpenEditor(undefined);
      update((state) => ({
        ...state,
        cpModal: {
          isOpen: true,
          mode: "create",
          form: { ...editor.form },
          snapshot: { ...editor.form },
          loading: false,
          confirmClose: false,
        },
      }));
    },
    async openCpEdit(counterpartyId: string): Promise<void> {
      const editor = await counterpartyOpenEditor(counterpartyId);
      update((state) => ({
        ...state,
        cpModal: {
          isOpen: true,
          mode: "edit",
          form: { ...editor.form },
          snapshot: { ...editor.form },
          loading: false,
          confirmClose: false,
        },
      }));
    },
    updateCpField(field: keyof CounterpartyDraftFormDto, value: string): void {
      update((state) => {
        if (!state.cpModal) return state;
        return {
          ...state,
          cpModal: { ...state.cpModal, form: { ...state.cpModal.form, [field]: value } },
        };
      });
    },
    async saveCp(): Promise<void> {
      const snap = get({ subscribe });
      if (!snap.cpModal) return;

      update((state) => ({
        ...state,
        cpModal: state.cpModal ? { ...state.cpModal, loading: true } : null,
      }));

      try {
        const result = await counterpartySave(snap.cpModal.form);
        const savedName =
          result.updatedList.find((cp) => cp.id === result.savedId)?.name ?? snap.cpModal.form.name;

        update((state) => {
          if (!state.editor) return { ...state, cpModal: null };

          const isCreate = snap.cpModal!.mode === "create";
          const updatedFormFields = isCreate
            ? { counterpartyId: result.savedId, counterpartyName: savedName }
            : { counterpartyName: savedName };

          const updatedEditorForm = { ...state.editor.form, ...updatedFormFields };

          return {
            ...state,
            editor: { ...state.editor, form: updatedEditorForm },
            editorSnapshot: state.editorSnapshot
              ? {
                  ...state.editorSnapshot,
                  form: { ...state.editorSnapshot.form, ...updatedFormFields },
                }
              : null,
            cpModal: null,
          };
        });
      } catch (error) {
        update((state) => ({
          ...state,
          cpModal: state.cpModal ? { ...state.cpModal, loading: false } : null,
          error: String(error),
        }));
      }
    },
    closeCpModal(): void {
      const snap = get({ subscribe });
      if (!snap.cpModal) return;
      const dirty = isEditorFormDirty(snap.cpModal.snapshot, snap.cpModal.form);
      if (dirty) {
        update((state) => ({
          ...state,
          cpModal: state.cpModal ? { ...state.cpModal, confirmClose: true } : null,
        }));
      } else {
        update((state) => ({ ...state, cpModal: null }));
      }
    },
    confirmCloseCpModal(): void {
      update((state) => ({ ...state, cpModal: null }));
    },
    cancelCloseCpModal(): void {
      update((state) => ({
        ...state,
        cpModal: state.cpModal ? { ...state.cpModal, confirmClose: false } : null,
      }));
    },
```

- [ ] **Step 5: Run cpModal tests to verify they pass**

```bash
cd frontend && npx vitest run --config vitest.config.mjs -t "documentsStore cpModal" 2>&1 | tail -15
```
Expected: all 7 tests pass.

- [ ] **Step 6: Run full frontend test suite**

```bash
cd frontend && npm run test:frontend 2>&1 | tail -5
```
Expected: all existing tests still pass.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/stores/documents.ts \
        frontend/src/lib/stores/__tests__/documents.test.ts
git commit -m "feat(documents): add cpModal state slice to documents store"
```

---

## Task 5: Documents store — changeCounterparty (TDD)

**Files:**
- Modify: `frontend/src/lib/stores/__tests__/documents.test.ts`
- Modify: `frontend/src/lib/stores/documents.ts`

- [ ] **Step 1: Write failing test**

In `frontend/src/lib/stores/__tests__/documents.test.ts`, add a new `describe` block after the cpModal suite:

```typescript
describe("documentsStore changeCounterparty", () => {
  beforeEach(() => {
    documentChangeCounterpartyMock.mockReset();
  });

  it("calls documentChangeCounterparty with correct args", async () => {
    documentChangeCounterpartyMock.mockResolvedValue({
      ok: true,
      counterpartyId: "cp-2",
      counterpartyName: "ФОП Петренко",
    });

    await documentsStore.changeCounterparty("act:some-uuid", "cp-2");

    expect(documentChangeCounterpartyMock).toHaveBeenCalledWith("act:some-uuid", "cp-2");
  });

  it("updates editor.form when editor is open", async () => {
    // Put store into editor state via mocked document open
    const mockDocOpenResult = api.documentOpen as ReturnType<typeof vi.fn>;
    const mockChainGet = api.documentChainGet as ReturnType<typeof vi.fn>;
    mockDocOpenResult.mockResolvedValue({
      form: {
        id: "act:some-uuid",
        kind: "act",
        number: "АКТ-001",
        date: "2026-05-01",
        counterpartyId: "cp-1",
        counterpartyName: "ТОВ Ромашка",
        direction: "outgoing",
        notes: "",
        status: "draft",
        statusLabel: "Чернетка",
      },
      items: [],
      pdf: null,
    });
    mockChainGet.mockResolvedValue({ items: [] });
    await documentsStore.open("act:some-uuid");

    documentChangeCounterpartyMock.mockResolvedValue({
      ok: true,
      counterpartyId: "cp-2",
      counterpartyName: "ФОП Петренко",
    });
    await documentsStore.changeCounterparty("act:some-uuid", "cp-2");

    const state = getState();
    expect(state.editor?.form.counterpartyId).toBe("cp-2");
    expect(state.editor?.form.counterpartyName).toBe("ФОП Петренко");
    expect(state.editorSnapshot?.form.counterpartyId).toBe("cp-2");
    expect(state.editorSnapshot?.form.counterpartyName).toBe("ФОП Петренко");
  });
});

- [ ] **Step 2: Run test to verify it fails**

```bash
cd frontend && npx vitest run --config vitest.config.mjs -t "documentsStore changeCounterparty" 2>&1 | tail -10
```
Expected: `FAIL` — `changeCounterparty` function does not exist.

- [ ] **Step 3: Add changeCounterparty to documents.ts**

In the store object in `frontend/src/lib/stores/documents.ts`, add after `cancelCloseCpModal`:

```typescript
    async changeCounterparty(docId: string, counterpartyId: string): Promise<void> {
      try {
        const result = await documentChangeCounterparty(docId, counterpartyId);
        update((state) => {
          if (!state.editor) return state;
          const updatedFields = {
            counterpartyId: result.counterpartyId,
            counterpartyName: result.counterpartyName,
          };
          return {
            ...state,
            editor: {
              ...state.editor,
              form: { ...state.editor.form, ...updatedFields },
            },
            editorSnapshot: state.editorSnapshot
              ? {
                  ...state.editorSnapshot,
                  form: { ...state.editorSnapshot.form, ...updatedFields },
                }
              : null,
          };
        });
      } catch (error) {
        update((state) => ({ ...state, error: String(error) }));
      }
    },
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd frontend && npx vitest run --config vitest.config.mjs -t "documentsStore changeCounterparty" 2>&1 | tail -10
```
Expected: tests pass.

- [ ] **Step 5: Run full test suite**

```bash
cd frontend && npm run test:frontend 2>&1 | tail -5
```

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/stores/documents.ts \
        frontend/src/lib/stores/__tests__/documents.test.ts
git commit -m "feat(documents): add changeCounterparty to documents store"
```

---

## Task 6: DocumentsScreen UI

**Files:**
- Modify: `frontend/src/lib/screens/DocumentsScreen.svelte`

- [ ] **Step 1: Add CounterpartyModal import**

In `frontend/src/lib/screens/DocumentsScreen.svelte`, after line 4 (`import AppIcon from "../components/AppIcon.svelte";`), add:

```svelte
  import CounterpartyModal from "../components/CounterpartyModal.svelte";
```

- [ ] **Step 2: Add local variables for reassignment mode**

In the `<script>` block of `DocumentsScreen.svelte`, after line 39 (after `let filterButton...`), add:

```typescript
  let isReassigning = false;
  let reassignTargetId = "";

  $: if ($documents.editor?.form.counterpartyId && !$documents.pendingNew) {
    isReassigning = false;
    reassignTargetId = $documents.editor.form.counterpartyId;
  }
```

Note: this reactive statement resets `isReassigning` whenever the editor's counterpartyId changes (e.g., after `changeCounterparty` succeeds).

- [ ] **Step 3: Add isDirtyCpModal derived value**

After the reactive statement above, add:

```typescript
  $: isDirtyCpModal = (() => {
    const m = $documents.cpModal;
    if (!m?.form || !m.snapshot) return false;
    return JSON.stringify(m.form) !== JSON.stringify(m.snapshot);
  })();
```

- [ ] **Step 4: Update onEditorCounterpartyChange to handle __new__**

Replace the existing `onEditorCounterpartyChange` function (lines 435–440):

```typescript
  function onEditorCounterpartyChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    const id = select.value;
    if (id === "__new__") {
      select.value = "";
      void documents.openCpCreate();
      return;
    }
    const name = select.options[select.selectedIndex]?.text ?? "";
    documents.updateCounterparty(id, id ? name : "");
  }
```

- [ ] **Step 5: Update the counterparty section in the editor template**

Replace lines 1161–1182 (the `{#if $documents.pendingNew}` ... `{/if}` block for counterparty):

```svelte
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
            <option value="__new__">+ Новий контрагент...</option>
          </select>
        </label>
      {:else if isReassigning}
        <div class="editor-field-readonly editor-grid-span">
          <span class="editor-field-readonly-label">Контрагент</span>
          <div class="cp-reassign-row">
            <select bind:value={reassignTargetId} disabled={$documents.loading}>
              {#each $counterparties.screen?.items ?? [] as cp}
                <option value={cp.id}>{cp.name}</option>
              {/each}
            </select>
            <button
              class="btn-primary"
              disabled={$documents.loading || !reassignTargetId || reassignTargetId === $documents.editor.form.counterpartyId}
              on:click={() => void documents.changeCounterparty($documents.editor.form.id, reassignTargetId)}
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
          <span class="editor-field-readonly-value">{$documents.editor.form.counterpartyName}</span>
          <div class="cp-actions">
            <button
              class="btn-ghost btn-sm"
              on:click={() => void documents.openCpEdit($documents.editor.form.counterpartyId)}
              disabled={$documents.loading}
            >
              Редагувати
            </button>
            <button
              class="btn-ghost btn-sm"
              on:click={() => { isReassigning = true; reassignTargetId = $documents.editor.form.counterpartyId; }}
              disabled={$documents.loading}
            >
              Змінити
            </button>
          </div>
        </div>
      {/if}
```

- [ ] **Step 6: Mount CounterpartyModal at end of editor section**

In `DocumentsScreen.svelte`, find the closing `</section>` tag of the editor (around line 1354, after the PDF section). Insert before that `</section>`:

```svelte
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
```

- [ ] **Step 7: Add CSS for new elements**

In the `<style>` block of `DocumentsScreen.svelte` (after line 1357), add:

```css
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

  .btn-sm {
    font-size: 12px;
    padding: 3px 10px;
  }
```

- [ ] **Step 8: svelte-check**

```bash
cd frontend && npm run check 2>&1 | tail -10
```
Expected: `0 errors`.

- [ ] **Step 9: Run full frontend test suite**

```bash
cd frontend && npm run test:frontend 2>&1 | tail -5
```

- [ ] **Step 10: Commit**

```bash
git add frontend/src/lib/screens/DocumentsScreen.svelte
git commit -m "feat(documents): add counterparty edit/create/reassign UI to document drawer"
```

---

## Task 7: Rust — ChangeCounterpartyResultDto and business logic

**Files:**
- Modify: `src/tauri_api/documents/api.rs`

- [ ] **Step 1: Add ChangeCounterpartyResultDto struct**

In `src/tauri_api/documents/api.rs`, find where other public DTOs are defined (near the top of the file). Add:

```rust
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCounterpartyResultDto {
    pub ok: bool,
    pub counterparty_id: String,
    pub counterparty_name: String,
}
```

- [ ] **Step 2: Add document_change_counterparty function**

In `src/tauri_api/documents/api.rs`, add after the `document_open` function:

```rust
pub async fn document_change_counterparty(
    ctx: &AppCtx,
    doc_id: String,
    counterparty_id: String,
) -> Result<ChangeCounterpartyResultDto> {
    let doc_ref = parse_document_ref(&doc_id)
        .ok_or_else(|| anyhow::anyhow!("Некоректний ідентифікатор документа: {}", doc_id))?;
    let cp_uuid = uuid::Uuid::parse_str(&counterparty_id)
        .map_err(|_| anyhow::anyhow!("Некоректний UUID контрагента: {}", counterparty_id))?;
    let company_id = ctx.company_id();
    let pool = ctx.pool();

    match doc_ref {
        DocumentRef::Act(id) => {
            sqlx::query!(
                "UPDATE acts SET counterparty_id = $1, updated_at = now() \
                 WHERE id = $2 AND company_id = $3",
                cp_uuid,
                id,
                company_id
            )
            .execute(pool)
            .await?;
        }
        DocumentRef::Invoice(id) => {
            sqlx::query!(
                "UPDATE invoices SET counterparty_id = $1, updated_at = now() \
                 WHERE id = $2 AND company_id = $3",
                cp_uuid,
                id,
                company_id
            )
            .execute(pool)
            .await?;
        }
        DocumentRef::Waybill(id) => {
            sqlx::query!(
                "UPDATE waybills SET counterparty_id = $1, updated_at = now() \
                 WHERE id = $2 AND company_id = $3",
                cp_uuid,
                id,
                company_id
            )
            .execute(pool)
            .await?;
        }
    }

    let cp_name = sqlx::query_scalar!(
        "SELECT name FROM counterparties WHERE id = $1 AND company_id = $2",
        cp_uuid,
        company_id
    )
    .fetch_one(pool)
    .await?;

    Ok(ChangeCounterpartyResultDto {
        ok: true,
        counterparty_id,
        counterparty_name: cp_name,
    })
}
```

- [ ] **Step 3: Run cargo sqlx prepare (requires DATABASE_URL)**

```bash
DATABASE_URL=postgres://postgres:password@localhost:5432/acta cargo sqlx prepare
```
Expected: `.sqlx/*.json` files updated. Commit them with the code.

If DB is not available, use runtime-style instead of `sqlx::query!` macros:

```rust
sqlx::query(
    "UPDATE acts SET counterparty_id = $1, updated_at = now() \
     WHERE id = $2 AND company_id = $3"
)
.bind(cp_uuid)
.bind(id)
.bind(company_id)
.execute(pool)
.await?;
```
And for the SELECT:
```rust
let cp_name: String = sqlx::query_scalar(
    "SELECT name FROM counterparties WHERE id = $1 AND company_id = $2"
)
.bind(cp_uuid)
.bind(company_id)
.fetch_one(pool)
.await?;
```

- [ ] **Step 4: Verify lib compiles**

```bash
cargo build --lib 2>&1 | tail -10
```
Expected: `Finished` with no errors.

- [ ] **Step 5: Commit**

```bash
git add src/tauri_api/documents/api.rs .sqlx/
git commit -m "feat(documents): add document_change_counterparty Rust function"
```

---

## Task 8: Tauri command and registration

**Files:**
- Modify: `src-tauri/src/commands/documents.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add Tauri command to documents.rs**

In `src-tauri/src/commands/documents.rs`, update the import block at the top to include `ChangeCounterpartyResultDto`:

```rust
use acta::tauri_api::documents::{
    BulkDocumentRequest, BulkMutationResultDto, ChangeCounterpartyResultDto, CreateChainDraftRequest,
    CreateDocumentDraftRequest, DocumentChainDto, DocumentEditorDto, DocumentPdfActionResultDto,
    DocumentsListDto, DocumentsListRequest, MutationResultDto, ReplaceDocumentPdfTextRequest,
    SaveDocumentRequest, SaveDocumentResponse,
};
```

Then add the new command function after `document_delete`:

```rust
#[tauri::command]
pub async fn document_change_counterparty(
    state: State<'_, TauriState>,
    doc_id: String,
    counterparty_id: String,
) -> CommandResult<ChangeCounterpartyResultDto> {
    acta::tauri_api::documents::document_change_counterparty(
        &state.ctx,
        doc_id,
        counterparty_id,
    )
    .await
    .map_err(|error| error.to_string())
}
```

- [ ] **Step 2: Register command in lib.rs**

In `src-tauri/src/lib.rs`, find the `tauri::generate_handler![` block (line 43). The document commands are listed there. Find `commands::documents::document_delete,` and add the new command after it:

```rust
            commands::documents::document_change_counterparty,
```

- [ ] **Step 3: Build the full Tauri project**

```bash
cargo build --tests 2>&1 | tail -10
```
Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/documents.rs src-tauri/src/lib.rs
git commit -m "feat(documents): register document_change_counterparty Tauri command"
```

---

## Task 9: Acceptance criterion — Rust integration test

**Files:**
- Modify: `tests/db_integration.rs` (or create `tests/document_change_counterparty.rs`)

The test verifies all 3 document types. Find the existing test infrastructure in `tests/db_integration.rs` to understand the pool setup pattern (typically `setup_test_pool()` or `PgPool::connect(TEST_DATABASE_URL)`).

- [ ] **Step 1: Add test module**

Add to `tests/db_integration.rs` (or a new test file):

```rust
#[cfg(test)]
mod document_change_counterparty_tests {
    use acta::tauri_api::documents::document_change_counterparty;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn get_pool() -> PgPool {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for DB integration tests");
        PgPool::connect(&url).await.expect("Failed to connect to test DB")
    }

    // Helper: insert a minimal counterparty and return its UUID
    async fn insert_counterparty(pool: &PgPool, company_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO counterparties (id, company_id, name, edrpou, created_at, updated_at) \
             VALUES ($1, $2, 'Test CP', '00000000', now(), now())",
            id, company_id
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    // Note: these tests require a seeded test DB with a company row.
    // Run: TEST_DATABASE_URL=... cargo test --test db_integration document_change_counterparty

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL and seeded DB"]
    async fn changes_counterparty_on_act() {
        let pool = get_pool().await;
        // Use existing seeded act and counterparty IDs from the test DB
        // Replace these with real UUIDs from your test seed data
        let company_id: Uuid = "your-company-uuid".parse().unwrap();
        let cp_b_id = insert_counterparty(&pool, company_id).await;
        let act_id: Uuid = "existing-act-uuid".parse().unwrap();
        let doc_id = format!("act:{}", act_id);

        let ctx = acta::AppCtx::new_for_test(pool.clone(), company_id);
        let result = document_change_counterparty(&ctx, doc_id, cp_b_id.to_string())
            .await
            .expect("should succeed");

        assert!(result.ok);
        assert_eq!(result.counterparty_id, cp_b_id.to_string());

        // Verify DB was updated
        let row = sqlx::query!("SELECT counterparty_id FROM acts WHERE id = $1", act_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.counterparty_id, cp_b_id);
    }
}
```

**Important:** The exact `AppCtx::new_for_test` constructor and seed UUIDs depend on your test infrastructure. Check `tests/db_integration.rs` for the existing pattern — copy the pool/ctx setup from there. The key acceptance criterion is: the function executes without error and the DB row is updated for all 3 types.

- [ ] **Step 2: Verify function reference compiles (no-DB check)**

```bash
cargo build --tests 2>&1 | tail -5
```
Expected: `Finished`.

- [ ] **Step 3: Run integration tests (requires TEST_DATABASE_URL)**

```bash
TEST_DATABASE_URL=postgres://postgres:password@localhost:5432/acta_test \
  cargo test --test db_integration document_change_counterparty 2>&1 | tail -10
```
Expected: test passes (or is skipped if `#[ignore]` is used until seed data is wired up).

- [ ] **Step 4: Commit**

```bash
git add tests/
git commit -m "test(documents): add acceptance criterion for document_change_counterparty across all doc types"
```

---

## Final verification

- [ ] **Run all frontend tests**

```bash
cd frontend && npm run test:frontend 2>&1 | tail -5
```

- [ ] **Run full Rust build**

```bash
cargo build --tests 2>&1 | tail -5
```

- [ ] **svelte-check**

```bash
cd frontend && npm run check 2>&1 | tail -5
```
