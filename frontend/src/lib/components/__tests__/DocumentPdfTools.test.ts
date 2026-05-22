/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import type { ComponentProps } from "svelte";
import DocumentPdfTools from "../documents/DocumentPdfTools.svelte";
import type { DocumentEditorDto } from "../../types";

const pdf: NonNullable<DocumentEditorDto["pdf"]> = {
  filePath: "C:/tmp/working.pdf",
  pageCount: 1,
  extractedText: "DRAFT STATUS",
  hasTextOps: true,
  editable: true,
  warnings: []
};

function mount(props: Partial<ComponentProps<DocumentPdfTools>> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new DocumentPdfTools({
    target,
    props: {
      documentId: "doc-1",
      pdf,
      loading: false,
      ...props
    } as ComponentProps<DocumentPdfTools>
  });

  return { component, target };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("DocumentPdfTools", () => {
  it("renders preview and dispatches PDF actions", async () => {
    const { component, target } = mount();
    const attachExistingPdf = vi.fn();
    const openCurrentPdf = vi.fn();
    const applyTextReplace = vi.fn();

    component.$on("attachExistingPdf", attachExistingPdf);
    component.$on("openCurrentPdf", openCurrentPdf);
    component.$on("applyTextReplace", applyTextReplace);

    expect(target.querySelector('[data-testid="documents-existing-pdf"]')).toBeTruthy();
    expect((target.querySelector("textarea[readonly]") as HTMLTextAreaElement).value).toBe("DRAFT STATUS");

    const buttons = Array.from(target.querySelectorAll("button"));
    buttons.find((button) => button.textContent?.includes("Прив'язати інший PDF"))?.click();
    buttons.find((button) => button.textContent?.includes("Відкрити PDF"))?.click();

    const inputs = target.querySelectorAll("input");
    (inputs[0] as HTMLInputElement).value = "DRAFT";
    inputs[0].dispatchEvent(new Event("input", { bubbles: true }));
    (inputs[1] as HTMLInputElement).value = "PAID";
    inputs[1].dispatchEvent(new Event("input", { bubbles: true }));
    await tick();

    buttons.find((button) => button.textContent?.includes("Застосувати exact replace"))?.click();

    expect(attachExistingPdf).toHaveBeenCalledOnce();
    expect(openCurrentPdf).toHaveBeenCalledOnce();
    expect(applyTextReplace).toHaveBeenCalledWith(expect.objectContaining({
      detail: { findText: "DRAFT", replaceText: "PAID" }
    }));

    component.$destroy();
  });

  it("disables exact replace when PDF is not editable and resets draft on document change", async () => {
    const { component, target } = mount({
      pdf: { ...pdf, editable: false, warnings: ["Лише перегляд."] }
    });

    expect(target.textContent).toContain("Лише перегляд.");
    expect(Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Застосувати exact replace")
    )?.disabled).toBe(true);

    component.$set({ pdf, documentId: "doc-2" });
    await tick();

    const inputs = target.querySelectorAll("input");
    expect((inputs[0] as HTMLInputElement).value).toBe("");
    expect((inputs[1] as HTMLInputElement).value).toBe("");

    component.$destroy();
  });
});
