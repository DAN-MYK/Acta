import { describe, expect, it } from "vitest";
import {
  getPaymentCandidateHint,
  getPaymentDirectionLabel,
  getPaymentDocumentKindLabel,
  getPaymentPreviewCopy,
  getPaymentStateLabel
} from "../paymentsPresentation";
import type { PaymentMatchPreviewDto } from "../types";

function makePreview(decisionKind: PaymentMatchPreviewDto["decisionKind"]): PaymentMatchPreviewDto {
  return {
    paymentId: "payment-1",
    isReconciled: false,
    decisionKind,
    candidates: [],
    autoMatch: null
  };
}

describe("paymentsPresentation", () => {
  it("formats payment state labels from canonical payments copy", () => {
    expect(getPaymentStateLabel("")).toBe("Не зведено");
    expect(getPaymentStateLabel("ACT-9")).toBe("Зв'язано з ACT-9");
  });

  it("resolves document kind and direction labels for payment presentation", () => {
    expect(getPaymentDocumentKindLabel("invoice")).toBe("Рахунок");
    expect(getPaymentDocumentKindLabel("act")).toBe("Акт");
    expect(getPaymentDirectionLabel("in")).toBe("Надходження");
    expect(getPaymentDirectionLabel("income")).toBe("Надходження");
    expect(getPaymentDirectionLabel("out")).toBe("Витрата");
    expect(getPaymentDirectionLabel("expense")).toBe("Витрата");
  });

  it("returns preview copy by decision kind and null without preview", () => {
    expect(getPaymentPreviewCopy(null)).toBeNull();
    expect(getPaymentPreviewCopy(makePreview("exact"))).toEqual({
      title: "Рекомендована звірка",
      description:
        "Система знайшла найкращий документ для автозіставлення. Перевірте рекомендацію перед підтвердженням."
    });
    expect(getPaymentPreviewCopy(makePreview("none"))?.title).toBe(
      "Автоматична звірка не знайшла точного документа"
    );
  });

  it("builds compact candidate hints from matching signals", () => {
    expect(
      getPaymentCandidateHint({
        documentId: "act-1",
        documentKind: "act",
        title: "ACT-001",
        openAmountStr: "8 450,00 грн",
        totalScore: 98,
        sameIban: true,
        referenceHit: true,
        textHits: 2,
        daysDistance: 0
      })
    ).toBe("той самий IBAN • є збіг по призначенню • текстових збігів: 2 • відхилення по даті: 0 дн.");
  });

  it("omits optional hint parts when a candidate has fewer signals", () => {
    expect(
      getPaymentCandidateHint({
        documentId: "invoice-1",
        documentKind: "invoice",
        title: "INV-001",
        openAmountStr: "4 000,00 грн",
        totalScore: 71,
        sameIban: false,
        referenceHit: false,
        textHits: 0,
        daysDistance: 3
      })
    ).toBe("відхилення по даті: 3 дн.");
  });
});
