import { get, writable } from "svelte/store";
import {
  paymentCreateOrUpdate,
  paymentMatchApplyAuto,
  paymentMatchManualCandidates,
  paymentMatchPreview,
  paymentReconcile,
  paymentsImportLatestCsv,
  paymentsList,
  paymentsOpenManualTemplate,
  paymentsSyncBank,
  paymentUnreconcileAll
} from "../api";
import type {
  MutationResultDto,
  OpenTemplateResultDto,
  PaymentDraftFormDto,
  PaymentItemDto,
  PaymentManualMatchCandidatesDto,
  PaymentMatchCandidateDto,
  PaymentMatchPreviewDto,
  PaymentsScreenDto
} from "../types";

type PaymentsActiveAction =
  | "import"
  | "sync"
  | "reconcile"
  | "manual-search"
  | "confirm-auto-match"
  | "confirm-candidate"
  | "confirm-manual-picker"
  | "confirm-split"
  | "unreconcile"
  | "save"
  | null;

interface PaymentManualPickerState {
  paymentId: string;
  query: string;
  candidates: PaymentMatchCandidateDto[];
  selectedCandidateId: string | null;
}

interface PaymentSplitAllocationDraft {
  documentId: string;
  documentKind: PaymentMatchCandidateDto["documentKind"];
  title: string;
  openAmountStr: string;
  amount: string;
}

interface PaymentSplitDraftState {
  paymentId: string;
  paymentAmountStr: string;
  remainingAmountStr: string;
  allocations: PaymentSplitAllocationDraft[];
}

interface PaymentsStoreState {
  list: PaymentsScreenDto | null;
  initialLoading: boolean;
  loading: boolean;
  error: string | null;
  editor: PaymentDraftFormDto | null;
  message: string | null;
  matchPreview: PaymentMatchPreviewDto | null;
  selectedCandidateId: string | null;
  manualPicker: PaymentManualPickerState | null;
  splitDraft: PaymentSplitDraftState | null;
  activeAction: PaymentsActiveAction;
  activePaymentId: string | null;
}

const initialState: PaymentsStoreState = {
  list: null,
  initialLoading: true,
  loading: false,
  error: null,
  editor: null,
  message: null,
  matchPreview: null,
  selectedCandidateId: null,
  manualPicker: null,
  splitDraft: null,
  activeAction: null,
  activePaymentId: null
};

function parseMoneyValue(value: string): number {
  const normalized = value
    .replace(/\u00a0/g, "")
    .replace(/\s+/g, "")
    .replace(/грн/gi, "")
    .replace(",", ".")
    .trim();
  const parsed = Number.parseFloat(normalized);
  return Number.isFinite(parsed) ? parsed : 0;
}

function formatMoneyValue(value: number): string {
  return new Intl.NumberFormat("uk-UA", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  }).format(Math.max(0, value));
}

function createPaymentsStore() {
  const { subscribe, update } = writable<PaymentsStoreState>(initialState);

  async function loadPayments() {
    update((state) => ({ ...state, loading: true, error: null }));
    try {
      const list = await paymentsList();
      update((state) => ({ ...state, list, initialLoading: false, loading: false }));
    } catch (error) {
      update((state) => ({ ...state, loading: false, error: String(error) }));
    }
  }

  async function refreshAfterMutation(message: string) {
    update((state) => ({ ...state, message }));
    await loadPayments();
  }

  const createBlankForm = (): PaymentDraftFormDto => ({
    id: "",
    date: "",
    amount: "",
    direction: "income",
    counterpartyId: "",
    counterpartyName: "",
    bankName: "",
    reference: "",
    description: ""
  });

  const clearPreview = (state: PaymentsStoreState): PaymentsStoreState => ({
    ...state,
    matchPreview: null,
    selectedCandidateId: null
  });

  const clearManualPicker = (state: PaymentsStoreState): PaymentsStoreState => ({
    ...state,
    manualPicker: null
  });

  const clearSplitDraft = (state: PaymentsStoreState): PaymentsStoreState => ({
    ...state,
    splitDraft: null
  });

  const beginAction = (action: PaymentsActiveAction, paymentId?: string) => {
    update((state) => ({
      ...state,
      loading: true,
      error: null,
      activeAction: action,
      activePaymentId: paymentId ?? null
    }));
  };

  const finishAction = () => {
    update((state) => ({
      ...state,
      loading: false,
      activeAction: null,
      activePaymentId: null
    }));
  };

  const getSelectedPreviewCandidate = (
    preview: PaymentMatchPreviewDto,
    selectedCandidateId: string | null
  ) => {
    if (preview.decisionKind !== "ambiguous" || !selectedCandidateId) {
      return null;
    }

    return preview.candidates.find((candidate) => candidate.documentId === selectedCandidateId) ?? null;
  };

  const getSelectedManualPickerCandidate = (manualPicker: PaymentManualPickerState | null) => {
    if (!manualPicker?.selectedCandidateId) {
      return null;
    }

    return (
      manualPicker.candidates.find((candidate) => candidate.documentId === manualPicker.selectedCandidateId) ?? null
    );
  };

  const toManualPickerState = (
    manualPicker: PaymentManualMatchCandidatesDto
  ): PaymentManualPickerState => ({
    paymentId: manualPicker.paymentId,
    query: manualPicker.query,
    candidates: manualPicker.candidates,
    selectedCandidateId: manualPicker.candidates[0]?.documentId ?? null
  });

  const recalculateSplitDraft = (
    splitDraft: PaymentSplitDraftState
  ): PaymentSplitDraftState => {
    const paymentAmount = parseMoneyValue(splitDraft.paymentAmountStr);
    const allocatedAmount = splitDraft.allocations.reduce(
      (sum, allocation) => sum + parseMoneyValue(allocation.amount),
      0
    );

    return {
      ...splitDraft,
      remainingAmountStr: formatMoneyValue(paymentAmount - allocatedAmount)
    };
  };

  const buildInitialSplitDraft = (
    state: PaymentsStoreState,
    paymentId: string,
    candidates: PaymentMatchCandidateDto[] = []
  ): PaymentSplitDraftState => {
    const paymentAmountStr = state.list?.items.find((item) => item.id === paymentId)?.amountStr;
    const fallbackAmount = candidates.reduce(
      (sum, candidate) => sum + parseMoneyValue(candidate.openAmountStr),
      0
    );
    const paymentAmount = parseMoneyValue(paymentAmountStr ?? "0");
    const resolvedAmount = formatMoneyValue(paymentAmount > 0 ? paymentAmount : fallbackAmount);

    return {
      paymentId,
      paymentAmountStr: resolvedAmount,
      remainingAmountStr: resolvedAmount,
      allocations: []
    };
  };

  const buildSplitDraftFromCandidates = (
    state: PaymentsStoreState,
    paymentId: string,
    candidates: PaymentMatchCandidateDto[]
  ): PaymentSplitDraftState => {
    const draft = buildInitialSplitDraft(state, paymentId, candidates);
    let remainingAmount = parseMoneyValue(draft.paymentAmountStr);

    draft.allocations = candidates
      .map((candidate) => {
        const allocationAmount = Math.min(
          Math.max(remainingAmount, 0),
          parseMoneyValue(candidate.openAmountStr)
        );
        remainingAmount -= allocationAmount;

        if (allocationAmount <= 0) {
          return null;
        }

        return {
          documentId: candidate.documentId,
          documentKind: candidate.documentKind,
          title: candidate.title,
          openAmountStr: candidate.openAmountStr,
          amount: formatMoneyValue(allocationAmount)
        };
      })
      .filter((allocation): allocation is PaymentSplitAllocationDraft => allocation !== null);

    return recalculateSplitDraft(draft);
  };

  return {
    subscribe,

    async load() {
      await loadPayments();
    },

    async importCsv(): Promise<MutationResultDto> {
      beginAction("import");
      try {
        const result = await paymentsImportLatestCsv();
        if (result.ok) {
          await refreshAfterMutation(result.message);
        } else {
          update((state) => ({ ...state, message: result.message, error: result.message }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      } finally {
        finishAction();
      }
    },

    async syncBank(): Promise<MutationResultDto> {
      beginAction("sync");
      try {
        const result = await paymentsSyncBank();
        if (result.ok) {
          await refreshAfterMutation(result.message);
        } else {
          update((state) => ({ ...state, message: result.message, error: result.message }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      } finally {
        finishAction();
      }
    },

    async reconcile(id: string) {
      beginAction("reconcile", id);
      try {
        const preview = await paymentMatchPreview({ paymentId: id });
        const selectedCandidateId = preview.candidates[0]?.documentId ?? preview.autoMatch?.documentId ?? null;

        let message = "Підберіть варіант звірки для платежу.";
        if (preview.decisionKind === "exact") {
          message = "Знайдено рекомендовану звірку. Перевірте та підтвердьте автозіставлення.";
        } else if (preview.decisionKind === "ambiguous") {
          message = "Знайдено кілька кандидатів. Цей платіж потребує уваги, а ручне підтвердження буде наступним кроком.";
        } else if (preview.decisionKind === "none") {
          message = "Точний кандидат не знайдено. Перевірте платіж або підготуйте ручне звіряння.";
        }

        if (preview.decisionKind === "split") {
          message =
            "Знайдено рекомендований розподіл платежу між кількома документами. Перевірте алокації перед підтвердженням.";
        }

        update((state) => ({
          ...state,
          loading: false,
          error: null,
          message,
          matchPreview: preview,
          selectedCandidateId,
          manualPicker: state.manualPicker?.paymentId === id ? state.manualPicker : null,
          splitDraft:
            preview.decisionKind === "split"
              ? buildSplitDraftFromCandidates(state, id, preview.candidates)
              : state.splitDraft?.paymentId === id
                ? state.splitDraft
                : null
        }));
      } catch (error) {
        const message = String(error);
        update((state) => ({
          ...clearSplitDraft(clearManualPicker(clearPreview(state))),
          loading: false,
          error: message,
          message
        }));
      } finally {
        update((state) => ({
          ...state,
          activeAction: null,
          activePaymentId: null
        }));
      }
    },

    async confirmPreviewAutoMatch(): Promise<MutationResultDto> {
      const preview = get({ subscribe }).matchPreview;

      if (!preview?.autoMatch) {
        const message = "Немає рекомендованої звірки для автоматичного підтвердження.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      beginAction("confirm-auto-match", preview.paymentId);
      try {
        const result = await paymentMatchApplyAuto({ paymentId: preview.paymentId });
        if (result.ok) {
          update((state) =>
            clearSplitDraft(clearManualPicker(clearPreview({ ...state, message: result.message })))
          );
          await loadPayments();
        } else {
          update((state) => ({ ...state, message: result.message, error: result.message }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      } finally {
        finishAction();
      }
    },

    async confirmSelectedPreviewCandidate(): Promise<MutationResultDto> {
      const { matchPreview, selectedCandidateId } = get({ subscribe });

      if (!matchPreview || matchPreview.decisionKind !== "ambiguous") {
        const message = "Ручне підтвердження доступне лише для preview з кількома кандидатами.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      const candidate = getSelectedPreviewCandidate(matchPreview, selectedCandidateId);
      if (!candidate) {
        const message = "Виберіть кандидата для підтвердження звірки.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      beginAction("confirm-candidate", matchPreview.paymentId);
      try {
        const result = await paymentReconcile({
          paymentId: matchPreview.paymentId,
          documentKind: candidate.documentKind,
          documentId: candidate.documentId,
          amount: candidate.openAmountStr
        });
        if (result.ok) {
          update((state) =>
            clearSplitDraft(clearManualPicker(clearPreview({ ...state, message: result.message })))
          );
          await loadPayments();
        } else {
          update((state) => ({ ...state, message: result.message, error: result.message }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      } finally {
        finishAction();
      }
    },

    selectPreviewCandidate(documentId: string) {
      update((state) => {
        if (!state.matchPreview) {
          return state;
        }

        return {
          ...state,
          selectedCandidateId: documentId,
          message: "Кандидата вибрано. Ручне підтвердження буде наступним кроком."
        };
      });
    },

    async openManualMatchPicker(paymentId: string, query = ""): Promise<MutationResultDto> {
      beginAction("manual-search", paymentId);
      try {
        const manualPicker = await paymentMatchManualCandidates({ paymentId, query });
        update((state) => ({
          ...state,
          loading: false,
          error: null,
          message: manualPicker.candidates.length
            ? "Оберіть документ зі списку або звузьте пошук."
            : "За цим запитом кандидатів не знайдено.",
          manualPicker: toManualPickerState(manualPicker),
          splitDraft:
            state.splitDraft?.paymentId === paymentId
              ? state.splitDraft
              : buildInitialSplitDraft(state, paymentId, manualPicker.candidates)
        }));
        return {
          ok: true,
          message: manualPicker.candidates.length
            ? "Кандидатів для ручної звірки оновлено."
            : "За цим запитом кандидатів не знайдено."
        };
      } catch (error) {
        const message = String(error);
        update((state) => ({
          ...clearSplitDraft(clearManualPicker(state)),
          loading: false,
          error: message,
          message
        }));
        return { ok: false, message };
      } finally {
        update((state) => ({
          ...state,
          activeAction: null,
          activePaymentId: null
        }));
      }
    },

    updateManualMatchQuery(query: string) {
      update((state) => {
        if (!state.manualPicker) {
          return state;
        }

        return {
          ...state,
          manualPicker: {
            ...state.manualPicker,
            query
          }
        };
      });
    },

    async searchManualMatchCandidates(): Promise<MutationResultDto> {
      const manualPicker = get({ subscribe }).manualPicker;
      if (!manualPicker) {
        const message = "Спершу відкрийте ручний пошук для цього платежу.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      return await this.openManualMatchPicker(manualPicker.paymentId, manualPicker.query);
    },

    selectManualPickerCandidate(documentId: string) {
      update((state) => {
        if (!state.manualPicker) {
          return state;
        }

        return {
          ...state,
          manualPicker: {
            ...state.manualPicker,
            selectedCandidateId: documentId
          },
          message: "Вибрано документ для ручного звіряння."
        };
      });
    },

    async addSelectedManualPickerCandidateToSplit(): Promise<MutationResultDto> {
      const { manualPicker, splitDraft } = get({ subscribe });
      const candidate = getSelectedManualPickerCandidate(manualPicker);

      if (!manualPicker || !splitDraft) {
        const message = "Спершу відкрийте ручний picker для розподілу платежу.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      if (!candidate) {
        const message = "Виберіть документ, який треба додати до розподілу.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      if (splitDraft.allocations.some((allocation) => allocation.documentId === candidate.documentId)) {
        const message = "Цей документ уже додано до розподілу.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      const remainingAmount = parseMoneyValue(splitDraft.remainingAmountStr);
      if (remainingAmount <= 0) {
        const message = "Увесь платіж уже розподілено. За потреби змініть суми в чернетці.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      const allocationAmount = Math.min(remainingAmount, parseMoneyValue(candidate.openAmountStr));

      update((state) => {
        if (!state.splitDraft) {
          return state;
        }

        return {
          ...state,
          splitDraft: recalculateSplitDraft({
            ...state.splitDraft,
            allocations: [
              ...state.splitDraft.allocations,
              {
                documentId: candidate.documentId,
                documentKind: candidate.documentKind,
                title: candidate.title,
                openAmountStr: candidate.openAmountStr,
                amount: formatMoneyValue(allocationAmount)
              }
            ]
          }),
          message: "Документ додано до чернетки розподілу.",
          error: null
        };
      });

      return { ok: true, message: "Документ додано до розподілу" };
    },

    updateSplitAllocationAmount(documentId: string, amount: string) {
      update((state) => {
        if (!state.splitDraft) {
          return state;
        }

        const current = state.splitDraft.allocations.find((allocation) => allocation.documentId === documentId);
        if (!current) {
          return state;
        }

        const nextAmount = parseMoneyValue(amount);
        const documentOpenAmount = parseMoneyValue(current.openAmountStr);
        const otherAllocationsTotal = state.splitDraft.allocations
          .filter((allocation) => allocation.documentId !== documentId)
          .reduce((sum, allocation) => sum + parseMoneyValue(allocation.amount), 0);
        const paymentAmount = parseMoneyValue(state.splitDraft.paymentAmountStr);

        if (nextAmount <= 0) {
          return {
            ...state,
            message: "Сума розподілу має бути більшою за нуль.",
            error: "Сума розподілу має бути більшою за нуль."
          };
        }

        if (nextAmount > documentOpenAmount) {
          return {
            ...state,
            message: "Сума розподілу не може перевищувати залишок документа.",
            error: "Сума розподілу не може перевищувати залишок документа."
          };
        }

        if (otherAllocationsTotal + nextAmount > paymentAmount) {
          return {
            ...state,
            message: "Сума розподілу не може перевищувати залишок платежу.",
            error: "Сума розподілу не може перевищувати залишок платежу."
          };
        }

        return {
          ...state,
          splitDraft: recalculateSplitDraft({
            ...state.splitDraft,
            allocations: state.splitDraft.allocations.map((allocation) =>
              allocation.documentId === documentId
                ? { ...allocation, amount: amount.trim() || allocation.amount }
                : allocation
            )
          }),
          error: null
        };
      });
    },

    removeSplitAllocation(documentId: string) {
      update((state) => {
        if (!state.splitDraft) {
          return state;
        }

        return {
          ...state,
          splitDraft: recalculateSplitDraft({
            ...state.splitDraft,
            allocations: state.splitDraft.allocations.filter(
              (allocation) => allocation.documentId !== documentId
            )
          }),
          message: "Документ прибрано з чернетки розподілу.",
          error: null
        };
      });
    },

    closeManualMatchPicker() {
      update((state) => ({
        ...clearManualPicker(state),
        message: null
      }));
    },

    async confirmManualPickerCandidate(): Promise<MutationResultDto> {
      const { manualPicker } = get({ subscribe });
      const candidate = getSelectedManualPickerCandidate(manualPicker);

      if (!manualPicker) {
        const message = "Ручний picker ще не відкрито.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      if (!candidate) {
        const message = "Виберіть документ для ручного звіряння.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      beginAction("confirm-manual-picker", manualPicker.paymentId);
      try {
        const result = await paymentReconcile({
          paymentId: manualPicker.paymentId,
          documentKind: candidate.documentKind,
          documentId: candidate.documentId,
          amount: candidate.openAmountStr
        });
        if (result.ok) {
          update((state) =>
            clearSplitDraft(clearManualPicker(clearPreview({ ...state, message: result.message })))
          );
          await loadPayments();
        } else {
          update((state) => ({ ...state, message: result.message, error: result.message }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      } finally {
        finishAction();
      }
    },

    async confirmSplitDraft(): Promise<MutationResultDto> {
      const { splitDraft } = get({ subscribe });

      if (!splitDraft) {
        const message = "Немає чернетки розподілу для підтвердження.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      if (splitDraft.allocations.length === 0) {
        const message = "Додайте хоча б один документ до розподілу.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      if (parseMoneyValue(splitDraft.remainingAmountStr) > 0) {
        const message = "Розподіл ще не завершено. Закрийте залишок платежу або зменште суму.";
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      }

      beginAction("confirm-split", splitDraft.paymentId);
      try {
        for (const allocation of splitDraft.allocations) {
          const result = await paymentReconcile({
            paymentId: splitDraft.paymentId,
            documentKind: allocation.documentKind,
            documentId: allocation.documentId,
            amount: allocation.amount
          });

          if (!result.ok) {
            update((state) => ({ ...state, message: result.message, error: result.message }));
            return result;
          }
        }

        const message = "Розподіл платежу підтверджено";
        update((state) =>
          clearSplitDraft(clearManualPicker(clearPreview({ ...state, message, error: null })))
        );
        await loadPayments();
        return { ok: true, message };
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      } finally {
        finishAction();
      }
    },

    async unreconcile(id: string) {
      beginAction("unreconcile", id);
      try {
        const result = await paymentUnreconcileAll({ paymentId: id });
        if (result.ok) {
          update((state) => clearSplitDraft(clearManualPicker(clearPreview(state))));
          await refreshAfterMutation(result.message);
        } else {
          update((state) => ({ ...state, message: result.message, error: result.message }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, message };
      } finally {
        finishAction();
      }
    },

    openEditor(payment?: PaymentItemDto) {
      if (!payment) {
        update((state) => ({
          ...state,
          editor: createBlankForm(),
          message: null
        }));
        return;
      }

      const editor: PaymentDraftFormDto = {
        id: payment.id,
        date: payment.date,
        amount: payment.amountStr,
        direction: payment.direction === "in" ? "income" : "expense",
        counterpartyId: payment.counterpartyId,
        counterpartyName: payment.counterparty,
        bankName: payment.account,
        reference: "",
        description: payment.matchedDoc
      };

      update((state) => ({
        ...state,
        editor,
        message: null
      }));
    },

    async openById(paymentId: string) {
      let state = get({ subscribe });
      if (!state.list) {
        await loadPayments();
        state = get({ subscribe });
      }

      let payment = state.list?.items.find((item) => item.id === paymentId);
      if (!payment) {
        await loadPayments();
        state = get({ subscribe });
        payment = state.list?.items.find((item) => item.id === paymentId);
      }

      if (!payment) {
        update((current) => ({
          ...current,
          error: "Платіж не знайдено у поточному списку",
          message: "Платіж не знайдено у поточному списку"
        }));
        return false;
      }

      this.openEditor(payment);
      return true;
    },

    closeEditor() {
      update((state) => ({
        ...state,
        editor: null,
        message: null
      }));
    },

    closeMatchPreview() {
      update((state) => ({
        ...clearSplitDraft(clearPreview(state)),
        message: null
      }));
    },

    updateFormField(field: keyof PaymentDraftFormDto, value: string) {
      update((state) => {
        if (!state.editor) {
          return state;
        }

        return {
          ...state,
          editor: {
            ...state.editor,
            [field]: value
          }
        };
      });
    },

    async save() {
      const editor = get({ subscribe }).editor;
      if (!editor) {
        return { ok: false, message: "Немає відкритого платежу для збереження" };
      }

      beginAction("save", editor.id || undefined);
      try {
        const result = await paymentCreateOrUpdate(editor);
        if (result.ok) {
          update((state) => ({
            ...state,
            message: result.message,
            editor: null
          }));
          await loadPayments();
        } else {
          update((state) => ({
            ...state,
            message: result.message,
            error: result.message
          }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({
          ...state,
          error: message,
          message
        }));
        return { ok: false, message };
      } finally {
        finishAction();
      }
    },

    async openManualTemplate(): Promise<OpenTemplateResultDto> {
      try {
        const result = await paymentsOpenManualTemplate();
        update((state) => ({ ...state, message: result.message }));
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message }));
        return { ok: false, path: "", message };
      }
    }
  };
}

export const paymentsStore = createPaymentsStore();
