import { get, writable } from "svelte/store";
import {
  paymentCreateOrUpdate,
  paymentMatchApplyAuto,
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
  PaymentMatchPreviewDto,
  PaymentsScreenDto
} from "../types";

type PaymentsActiveAction =
  | "import"
  | "sync"
  | "reconcile"
  | "confirm-auto-match"
  | "confirm-candidate"
  | "unreconcile"
  | "save"
  | null;

interface PaymentsStoreState {
  list: PaymentsScreenDto | null;
  loading: boolean;
  error: string | null;
  editor: PaymentDraftFormDto | null;
  message: string | null;
  matchPreview: PaymentMatchPreviewDto | null;
  selectedCandidateId: string | null;
  activeAction: PaymentsActiveAction;
  activePaymentId: string | null;
}

function createPaymentsStore() {
  const { subscribe, update } = writable<PaymentsStoreState>({
    list: null,
    loading: false,
    error: null,
    editor: null,
    message: null,
    matchPreview: null,
    selectedCandidateId: null,
    activeAction: null,
    activePaymentId: null
  });

  async function loadPayments() {
    update((state) => ({ ...state, loading: true, error: null }));
    try {
      const list = await paymentsList();
      update((state) => ({ ...state, list, loading: false }));
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

        update((state) => ({
          ...state,
          loading: false,
          error: null,
          message,
          matchPreview: preview,
          selectedCandidateId
        }));
      } catch (error) {
        const message = String(error);
        update((state) => ({
          ...clearPreview(state),
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
          update((state) => clearPreview({ ...state, message: result.message }));
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
          update((state) => clearPreview({ ...state, message: result.message }));
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

    closeMatchPreview() {
      update((state) => ({
        ...clearPreview(state),
        message: null
      }));
    },

    async unreconcile(id: string) {
      beginAction("unreconcile", id);
      try {
        const result = await paymentUnreconcileAll({ paymentId: id });
        if (result.ok) {
          update((state) => clearPreview(state));
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
        update((state) => ({ ...state, message, error: message }));
        return { ok: false, path: "", message };
      }
    }
  };
}

export const paymentsStore = createPaymentsStore();
