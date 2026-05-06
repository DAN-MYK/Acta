import { describe, expect, it } from "vitest";
import {
  DOCUMENT_KIND_FILTER_OPTIONS,
  EDITOR_DIRTY_COPY,
  PAYMENT_CALENDAR_COPY,
  PAYMENT_SCREEN_COPY,
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

  it("provides shared payments short copy for toolbar and empty states", () => {
    expect(PAYMENT_SCREEN_COPY.importStatement).toBe("Імпортувати виписку");
    expect(PAYMENT_SCREEN_COPY.confirmManualDocument).toBe("Підтвердити вибраний документ");
    expect(PAYMENT_SCREEN_COPY.emptyMatchedTitle).toBe("Ще немає зведених платежів");
    expect(PAYMENT_SCREEN_COPY.stateMatched("ACT-7")).toBe("Зв'язано з ACT-7");
  });

  it("provides shared payment-calendar short copy", () => {
    expect(PAYMENT_CALENDAR_COPY.title).toBe("Платіжний календар");
    expect(PAYMENT_CALENDAR_COPY.retryAction).toBe("Спробувати ще раз");
    expect(PAYMENT_CALENDAR_COPY.filterEmptyTitle).toBe("У цьому місяці немає подій для поточного фільтра");
  });
});
