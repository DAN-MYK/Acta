import { describe, expect, it } from "vitest";
import {
  getPaymentBusyFlags,
  getPaymentFlowCopy,
  getPaymentGroups,
  getManualPickerState
} from "../payments/payment-view-model";
import { PAYMENT_MANUAL_PICKER_DISABLED_REASON } from "../../config/ui";
import type { PaymentItemDto } from "../../types";

const unmatchedPayment: PaymentItemDto = {
  id: "payment-unmatched",
  date: "2026-05-20",
  counterpartyId: "counterparty-1",
  counterparty: "ТОВ Приклад",
  amountStr: "1 200,00",
  direction: "in",
  matchedDoc: "",
  account: "UA123"
};

const matchedPayment: PaymentItemDto = {
  id: "payment-matched",
  date: "2026-05-21",
  counterpartyId: "counterparty-2",
  counterparty: "ФОП Тест",
  amountStr: "-800,00",
  direction: "out",
  matchedDoc: "Акт ACT-001",
  account: "UA456"
};

describe("payment-view-model", () => {
  it("splits payments into unmatched and matched groups without mutating order", () => {
    const groups = getPaymentGroups([unmatchedPayment, matchedPayment]);

    expect(groups.unmatchedPayments).toEqual([unmatchedPayment]);
    expect(groups.matchedPayments).toEqual([matchedPayment]);
  });

  it("derives busy flags from the active payment action", () => {
    expect(getPaymentBusyFlags(true, "import-commit")).toMatchObject({
      busyImport: false,
      busyImportCommit: true,
      busySync: false
    });

    expect(getPaymentBusyFlags(false, "import-commit")).toMatchObject({
      busyImportCommit: false
    });
  });

  it("returns flow copy only while an action is loading", () => {
    expect(getPaymentFlowCopy(true, "sync")).toEqual({
      title: "Оновлюємо рухи з банку",
      description: "Підтягуємо свіжі банківські рухи та готуємо їх до наступного кроку звірки."
    });
    expect(getPaymentFlowCopy(false, "sync")).toBeNull();
    expect(getPaymentFlowCopy(true, null)).toBeNull();
  });

  it("exposes manual picker confirm state and disabled reason", () => {
    expect(
      getManualPickerState({
        paymentId: "payment-1",
        query: "",
        selectedCandidateId: null,
        candidates: []
      })
    ).toEqual({
      canConfirm: false,
      disabledReason: PAYMENT_MANUAL_PICKER_DISABLED_REASON
    });

    expect(
      getManualPickerState({
        paymentId: "payment-1",
        query: "",
        selectedCandidateId: "document-1",
        candidates: [
          {
            documentId: "document-1",
            documentKind: "act",
            title: "Акт ACT-001",
            openAmountStr: "1 200,00",
            totalScore: 0.9,
            sameIban: true,
            referenceHit: true,
            textHits: 2,
            daysDistance: 0
          }
        ]
      })
    ).toEqual({ canConfirm: true, disabledReason: "" });
  });
});
