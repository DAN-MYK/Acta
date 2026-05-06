import { describe, expect, it } from "vitest";
import {
  DOCUMENT_KIND_FILTER_OPTIONS,
  EDITOR_DIRTY_COPY,
  resolveDocumentKindMeta
} from "../config/ui";

describe("ui config", () => {
  it("provides canonical dirty-editor copy for shared drawers", () => {
    expect(EDITOR_DIRTY_COPY).toEqual({
      dirtyTitle: "У вас є незбережені зміни",
      dirtyDescription: "Скасувати їх і закрити форму?",
      dirtyStay: "Залишитися",
      dirtyDiscard: "Так, закрити"
    });
  });

  it("provides shared document-kind filter options", () => {
    expect(DOCUMENT_KIND_FILTER_OPTIONS).toEqual([
      { value: null, label: "Всі" },
      { value: "act", label: "Акти" },
      { value: "invoice", label: "Рахунки" },
      { value: "waybill", label: "Накладні" }
    ]);
  });

  it("resolves invoice meta from the canonical document-kind source", () => {
    expect(resolveDocumentKindMeta("invoice")).toMatchObject({
      label: "Рахунок",
      icon: "invoice"
    });
  });
});
