/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ComponentProps } from "svelte";
import DocumentItemsEditor from "../documents/DocumentItemsEditor.svelte";
import type { DocumentDraftItemDto } from "../../types";

const items: DocumentDraftItemDto[] = [
  {
    description: "Консультація",
    unit: "год",
    quantity: "2",
    price: "2500"
  },
  {
    description: "Підтримка",
    unit: "міс",
    quantity: "1.5",
    price: "1000.25"
  }
];

function mount(props: Partial<ComponentProps<DocumentItemsEditor>> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new DocumentItemsEditor({
    target,
    props: {
      items,
      loading: false,
      ...props
    } as ComponentProps<DocumentItemsEditor>
  });

  return { component, target };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("DocumentItemsEditor", () => {
  it("renders document items and dispatches row actions", async () => {
    const { component, target } = mount();
    const addItem = vi.fn();
    const removeItem = vi.fn();
    const updateItemField = vi.fn();

    component.$on("addItem", addItem);
    component.$on("removeItem", removeItem);
    component.$on("updateItemField", updateItemField);

    expect(target.textContent).toContain("Позиції документа");
    expect(target.textContent).toContain("6 500,38 грн");
    const descriptionInputs = Array.from(
      target.querySelectorAll('input[aria-label^="Опис рядка"]')
    ) as HTMLInputElement[];
    expect(descriptionInputs.map((input) => input.value)).toEqual(["Консультація", "Підтримка"]);

    (Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Додати позицію")
    ) as HTMLButtonElement).click();

    const descriptionInput = descriptionInputs[0];
    descriptionInput.value = "Оновлена консультація";
    descriptionInput.dispatchEvent(new Event("input", { bubbles: true }));

    (target.querySelector('button[aria-label="Прибрати рядок 2"]') as HTMLButtonElement).click();
    await Promise.resolve();

    expect(addItem).toHaveBeenCalledOnce();
    expect(updateItemField).toHaveBeenCalledWith(expect.objectContaining({
      detail: { index: 0, field: "description", value: "Оновлена консультація" }
    }));
    expect(removeItem).toHaveBeenCalledWith(expect.objectContaining({ detail: 1 }));

    component.$destroy();
  });

  it("renders empty state and disables controls while loading", () => {
    const { component, target } = mount({ items: [], loading: true });

    expect(target.querySelector('[data-testid="documents-items-empty"]')).toBeTruthy();
    expect(Array.from(target.querySelectorAll("button")).every((button) => button.disabled)).toBe(true);

    component.$destroy();
  });
});
