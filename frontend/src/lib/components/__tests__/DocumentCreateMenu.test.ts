/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ComponentProps } from "svelte";
import DocumentCreateMenu from "../documents/DocumentCreateMenu.svelte";

function mount(props: Partial<ComponentProps<DocumentCreateMenu>> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new DocumentCreateMenu({
    target,
    props: {
      open: false,
      loading: false,
      selectedKind: null,
      activeTab: "all",
      ...props
    } as ComponentProps<DocumentCreateMenu>
  });

  return { component, target };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("DocumentCreateMenu", () => {
  it("opens a picker when no document kind is selected", async () => {
    const { component, target } = mount();
    const toggle = vi.fn();
    const directCreate = vi.fn();
    component.$on("toggle", toggle);
    component.$on("directCreate", directCreate);

    (target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
    await Promise.resolve();

    expect(toggle).toHaveBeenCalledOnce();
    expect(directCreate).not.toHaveBeenCalled();

    component.$destroy();
  });

  it("dispatches directCreate when a document kind is already selected", async () => {
    const { component, target } = mount({ selectedKind: "act" });
    const directCreate = vi.fn();
    component.$on("directCreate", directCreate);

    (target.querySelector('[data-testid="documents-create-button"]') as HTMLButtonElement).click();
    await Promise.resolve();

    expect(directCreate).toHaveBeenCalledWith(expect.objectContaining({ detail: "act" }));

    component.$destroy();
  });

  it("dispatches menuCreate from the picker", async () => {
    const { component, target } = mount({ open: true });
    const menuCreate = vi.fn();
    component.$on("menuCreate", menuCreate);

    (target.querySelector('[data-testid="documents-create-picker-act"]') as HTMLButtonElement).click();
    await Promise.resolve();

    expect(menuCreate).toHaveBeenCalledWith(expect.objectContaining({ detail: "act" }));

    component.$destroy();
  });
});
