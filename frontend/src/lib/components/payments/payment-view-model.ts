import {
  PAYMENT_FLOW_COPY,
  PAYMENT_MANUAL_PICKER_DISABLED_REASON,
  type PaymentActiveAction
} from "../../config/ui";
import type { PaymentItemDto, PaymentMatchCandidateDto } from "../../types";

export interface PaymentManualPickerViewState {
  paymentId: string;
  query: string;
  candidates: PaymentMatchCandidateDto[];
  selectedCandidateId: string | null;
}

export interface PaymentSplitAllocationViewState {
  documentId: string;
  documentKind: PaymentMatchCandidateDto["documentKind"];
  title: string;
  openAmountStr: string;
  amount: string;
}

export interface PaymentSplitDraftViewState {
  paymentId: string;
  paymentAmountStr: string;
  remainingAmountStr: string;
  allocations: PaymentSplitAllocationViewState[];
}

export function getPaymentGroups(items: PaymentItemDto[]) {
  return {
    unmatchedPayments: items.filter((item) => !item.matchedDoc),
    matchedPayments: items.filter((item) => Boolean(item.matchedDoc))
  };
}

export function getPaymentBusyFlags(loading: boolean, activeAction: PaymentActiveAction | null) {
  return {
    busyImport: loading && activeAction === "import",
    busyImportPick: loading && activeAction === "import-pick",
    busyImportCommit: loading && activeAction === "import-commit",
    busySync: loading && activeAction === "sync"
  };
}

export function isPaymentBusy(loading: boolean, activePaymentId: string | null, paymentId: string): boolean {
  return loading && activePaymentId === paymentId;
}

export function getPaymentFlowCopy(loading: boolean, activeAction: PaymentActiveAction | null) {
  if (!loading || !activeAction) {
    return null;
  }

  return PAYMENT_FLOW_COPY[activeAction] ?? null;
}

export function getManualPickerState(manualPicker: PaymentManualPickerViewState | null) {
  const canConfirm = Boolean(
    manualPicker?.selectedCandidateId && (manualPicker?.candidates.length ?? 0) > 0
  );
  const disabledReason =
    !manualPicker || manualPicker.candidates.length > 0 ? "" : PAYMENT_MANUAL_PICKER_DISABLED_REASON;

  return { canConfirm, disabledReason };
}
