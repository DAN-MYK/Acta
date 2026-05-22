/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import type { ComponentProps } from "svelte";
import DocumentList from "../documents/DocumentList.svelte";
import type { DocumentsListDto } from "../../types";

const items: DocumentsListDto["items"] = [
  {
    id: "doc-1",
    kind: "invoice",
    number: "INV-7",
    date: "2026-04-30",
    counterparty: "ТОВ Ромашка",
    amountStr: "5 000,00 грн",
    direction: "outgoing",
    status: "draft",
    statusLabel: "Чернетка",
    linkedId: ""
  },
  {
    id: "doc-2",
    kind: "act",
    number: "ACT-9",
    date: "2026-05-01",
    counterparty: "ФОП Тест",
    amountStr: "2 500,00 грн",
    direction: "incoming",
    status: "issued",
    statusLabel: "Виставлено",
    linkedId: ""
  }
];

function mount(props: Partial<ComponentProps<DocumentList>> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new DocumentList({
    target,
    props: {
      items,
      selectedIds: [],
      loading: false,
      ...props
    } as ComponentProps<DocumentList>
  });

  return { component, target };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("DocumentList", () => {
  it("dispatches row, selection and bulk actions without nesting checkbox in the row button", async () => {
    const { component, target } = mount({ selectedIds: ["doc-1"] });
    const open = vi.fn();
    const toggleSelection = vi.fn();
    const bulkDelete = vi.fn();
    component.$on("open", open);
    component.$on("toggleSelection", toggleSelection);
    component.$on("bulkDelete", bulkDelete);

    expect(target.querySelector('[data-testid="documents-row-doc-1"] button input[type="checkbox"]')).toBeNull();

    (target.querySelector('[data-testid="documents-row-doc-1"] .doc-row-open') as HTMLButtonElement).click();
    (target.querySelector('[data-testid="documents-row-doc-1"] .doc-row-checkbox input') as HTMLInputElement).click();
    await Promise.resolve();

    expect(open).toHaveBeenCalledWith(expect.objectContaining({ detail: "doc-1" }));
    expect(toggleSelection).toHaveBeenCalledWith(expect.objectContaining({ detail: "doc-1" }));

    const deleteButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Видалити вибрані")
    ) as HTMLButtonElement;
    deleteButton.click();
    await Promise.resolve();

    expect(target.querySelector('[data-testid="documents-confirm-bulk-banner"]')).toBeTruthy();
    (target.querySelector('[data-testid="documents-confirm-bulk-confirm"]') as HTMLButtonElement).click();
    await Promise.resolve();

    expect(bulkDelete).toHaveBeenCalledOnce();
    component.$destroy();
  });

  it("drops pending bulk delete confirmation when selection disappears", async () => {
    const { component, target } = mount({ selectedIds: ["doc-1"] });
    const bulkDelete = vi.fn();
    component.$on("bulkDelete", bulkDelete);

    const deleteButton = target.querySelector('[data-testid="documents-bulk-actions"] .btn-danger') as HTMLButtonElement;
    deleteButton.click();
    await tick();

    expect(target.querySelector('[data-testid="documents-confirm-bulk-banner"]')).toBeTruthy();

    component.$set({ selectedIds: [] });
    await tick();

    expect(target.querySelector('[data-testid="documents-confirm-bulk-banner"]')).toBeNull();
    expect(bulkDelete).not.toHaveBeenCalled();

    component.$destroy();
  });
});
