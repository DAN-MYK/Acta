import { describe, expect, it } from "vitest";
import {
  DOCUMENT_KIND_FILTER_OPTIONS,
  EDITOR_DIRTY_COPY,
  formatCalendarEventsLabel,
  formatCalendarMoreEventsLabel,
  formatOverdueDaysLabel,
  getCalendarEventDirectionLabel,
  getPaymentDirectionLabel,
  PAYMENT_CALENDAR_COPY,
  PAYMENT_DIRECTION_OPTIONS,
  PAYMENT_SCREEN_COPY,
  resolveDocumentKindMeta,
  supportsDocumentPdfGeneration,
  supportsExistingPdfFlow
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

  it("keeps generated PDF capability explicit", () => {
    expect(supportsDocumentPdfGeneration("act")).toBe(true);
    expect(supportsDocumentPdfGeneration("invoice")).toBe(true);
    expect(supportsDocumentPdfGeneration("waybill")).toBe(false);
  });

  it("keeps existing PDF attach capability explicit", () => {
    expect(supportsExistingPdfFlow("invoice")).toBe(true);
    expect(supportsExistingPdfFlow("waybill")).toBe(true);
    expect(supportsExistingPdfFlow("act")).toBe(false);
  });

  it("provides shared payments short copy for toolbar and empty states", () => {
    expect(PAYMENT_SCREEN_COPY.importStatement).toBe("Імпортувати виписку");
    expect(PAYMENT_SCREEN_COPY.confirmManualDocument).toBe("Підтвердити вибраний документ");
    expect(PAYMENT_SCREEN_COPY.emptyMatchedTitle).toBe("Ще немає зведених платежів");
    expect(PAYMENT_SCREEN_COPY.stateMatched("ACT-7")).toBe("Зв'язано з ACT-7");
  });

  it("maps API and form direction values to one label source", () => {
    expect(getPaymentDirectionLabel("in")).toBe("Надходження");
    expect(getPaymentDirectionLabel("income")).toBe("Надходження");
    expect(getPaymentDirectionLabel("out")).toBe("Витрата");
    expect(getPaymentDirectionLabel("expense")).toBe("Витрата");
    expect(getCalendarEventDirectionLabel("income")).toBe("Надходження");
    expect(getCalendarEventDirectionLabel("expense")).toBe("Витрата");
  });

  it("exposes select options for the payment editor", () => {
    expect(PAYMENT_DIRECTION_OPTIONS).toEqual([
      { value: "income", label: "Надходження" },
      { value: "expense", label: "Витрата" }
    ]);
  });

  it("provides shared payment-calendar short copy", () => {
    expect(PAYMENT_CALENDAR_COPY.title).toBe("Платіжний календар");
    expect(PAYMENT_CALENDAR_COPY.retryAction).toBe("Спробувати ще раз");
    expect(PAYMENT_CALENDAR_COPY.filterEmptyTitle).toBe("У цьому місяці немає подій для поточного фільтра");
  });

  it("formats non-overdue and overdue day labels in Ukrainian", () => {
    expect(formatOverdueDaysLabel(0)).toBe("Без прострочки");
    expect(formatOverdueDaysLabel(-2)).toBe("Без прострочки");
    expect(formatOverdueDaysLabel(1)).toBe("Прострочено 1 день");
    expect(formatOverdueDaysLabel(2)).toBe("Прострочено 2 дні");
    expect(formatOverdueDaysLabel(5)).toBe("Прострочено 5 днів");
    expect(formatOverdueDaysLabel(21)).toBe("Прострочено 21 день");
  });

  it("formats calendar event count labels and compact more labels", () => {
    expect(formatCalendarEventsLabel(0)).toBe("без подій");
    expect(formatCalendarEventsLabel(1)).toBe("1 подія");
    expect(formatCalendarEventsLabel(2)).toBe("2 події");
    expect(formatCalendarEventsLabel(5)).toBe("5 подій");
    expect(formatCalendarEventsLabel(11)).toBe("11 подій");
    expect(formatCalendarEventsLabel(21)).toBe("21 подія");
    expect(formatCalendarEventsLabel(22)).toBe("22 події");
    expect(formatCalendarMoreEventsLabel(1)).toBe("+1 ще");
    expect(formatCalendarMoreEventsLabel(3)).toBe("+3 ще");
  });
});
