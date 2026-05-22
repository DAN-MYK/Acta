/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ComponentProps } from "svelte";
import DocumentEditorDrawer from "../documents/DocumentEditorDrawer.svelte";
import type { DocumentChainDto, DocumentEditorDto } from "../../types";

const editor: DocumentEditorDto = {
  form: {
    id: "doc-1",
    kind: "invoice",
    direction: "outgoing",
    counterpartyId: "counterparty-1",
    counterpartyName: "ТОВ Ромашка",
    title: "Рахунок INV-7",
    number: "INV-7",
    date: "2026-04-30",
    notes: "Початковий коментар"
  },
  items: [],
  pdf: null,
  showTypePicker: false,
  showEditor: true
};

const chain: DocumentChainDto = {
  sourceId: "doc-1",
  steps: [
    { docType: "invoice", docNumber: "INV-7", amountStr: "5 000,00 грн", status: "Чернетка", exists: true }
  ]
};

function mount(props: Partial<ComponentProps<DocumentEditorDrawer>> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new DocumentEditorDrawer({
    target,
    props: {
      editor,
      chain,
      pendingNew: false,
      loading: false,
      companyName: "ТОВ Акт",
      counterparties: [{ id: "counterparty-1", name: "ТОВ Ромашка" }],
      ...props
    } as ComponentProps<DocumentEditorDrawer>
  });

  return { component, target };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("DocumentEditorDrawer", () => {
  it("renders the drawer shell and dispatches header/form actions", async () => {
    const { component, target } = mount();
    const close = vi.fn();
    const save = vi.fn();
    const updateFormField = vi.fn();
    const deleteCurrent = vi.fn();

    component.$on("close", close);
    component.$on("save", save);
    component.$on("updateFormField", updateFormField);
    component.$on("deleteCurrent", deleteCurrent);

    expect(target.querySelector('[data-testid="documents-drawer"]')).toBeTruthy();
    expect(target.textContent).toContain("ТОВ Акт");
    expect(target.textContent).toContain("Рахунок INV-7");

    (target.querySelector('[data-testid="documents-drawer-backdrop"]') as HTMLButtonElement).click();
    (Array.from(target.querySelectorAll("button")).find((button) => button.textContent?.includes("Зберегти")) as HTMLButtonElement).click();
    (target.querySelector('input[name="direction"][value="incoming"]') as HTMLInputElement).click();
    (target.querySelector('[data-testid="documents-delete-current-btn"]') as HTMLButtonElement).click();
    await Promise.resolve();

    expect(close).toHaveBeenCalledOnce();
    expect(save).toHaveBeenCalledOnce();
    expect(updateFormField).toHaveBeenCalledWith(expect.objectContaining({
      detail: { field: "direction", value: "incoming" }
    }));
    expect(deleteCurrent).toHaveBeenCalledOnce();

    component.$destroy();
  });

  it("keeps dirty and delete confirmations in the drawer DOM contract", () => {
    const { component, target } = mount({ pendingDirtyClose: true, pendingDelete: true });

    expect(target.querySelector('[data-testid="documents-dirty-banner"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="documents-confirm-delete-banner"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="documents-confirm-delete-confirm"]')).toBeTruthy();

    component.$destroy();
  });
});
