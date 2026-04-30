import { get, writable } from "svelte/store";
import {
  paymentsList,
  paymentsImportLatestCsv,
  paymentsSyncBank,
  paymentsOpenManualTemplate,
  paymentCreateOrUpdate,
  paymentReconcile,
  paymentUnreconcile
} from "../api";
import type {
  MutationResultDto,
  OpenTemplateResultDto,
  PaymentDraftFormDto,
  PaymentItemDto,
  PaymentsScreenDto
} from "../types";

interface PaymentsStoreState {
  list: PaymentsScreenDto | null;
  loading: boolean;
  error: string | null;
  editor: PaymentDraftFormDto | null;
  message: string | null;
}

function createPaymentsStore() {
  const { subscribe, update } = writable<PaymentsStoreState>({
    list: null,
    loading: false,
    error: null,
    editor: null,
    message: null
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

  return {
    subscribe,

    async load() {
      await loadPayments();
    },

    async importCsv(): Promise<MutationResultDto> {
      try {
        const result = await paymentsImportLatestCsv();
        if (result.ok) {
          await refreshAfterMutation(result.message);
        } else {
          update((state) => ({ ...state, message: result.message }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message }));
        return { ok: false, message };
      }
    },

    async syncBank(): Promise<MutationResultDto> {
      try {
        const result = await paymentsSyncBank();
        if (result.ok) {
          await refreshAfterMutation(result.message);
        } else {
          update((state) => ({ ...state, message: result.message }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({ ...state, message }));
        return { ok: false, message };
      }
    },

    async reconcile(id: string) {
      try {
        const result = await paymentReconcile(id);
        if (result.ok) {
          await refreshAfterMutation(result.message);
        }
      } catch (error) {
        update((state) => ({ ...state, message: String(error) }));
      }
    },

    async unreconcile(id: string) {
      try {
        const result = await paymentUnreconcile(id);
        if (result.ok) {
          await refreshAfterMutation(result.message);
        }
      } catch (error) {
        update((state) => ({ ...state, message: String(error) }));
      }
    },

    openEditor(payment?: PaymentItemDto) {
      if (!payment) {
        update((state) => ({
          ...state,
          editor: createBlankForm(),
          message: null
        }));
      } else {
        const editor: PaymentDraftFormDto = {
          id: payment.id,
          date: payment.date,
          amount: payment.amountStr,
          direction: payment.direction === "in" ? "income" : "expense",
          counterpartyId: "",
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
      }
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
        if (state.editor) {
          return {
            ...state,
            editor: {
              ...state.editor,
              [field]: value
            }
          };
        }
        return state;
      });
    },

    async save() {
      const editor = get({ subscribe }).editor;
      if (!editor) {
        return { ok: false, message: "Немає відкритого платежу для збереження" };
      }

      update((state) => ({ ...state, loading: true, error: null }));

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
            loading: false,
            error: result.message
          }));
        }
        return result;
      } catch (error) {
        const message = String(error);
        update((state) => ({
          ...state,
          loading: false,
          error: message,
          message: message
        }));
        return { ok: false, message };
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
