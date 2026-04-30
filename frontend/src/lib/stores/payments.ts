import { writable } from "svelte/store";
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
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        const list = await paymentsList();
        update((state) => ({ ...state, list, loading: false }));
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
      }
    },

    async importCsv(): Promise<MutationResultDto> {
      try {
        const result = await paymentsImportLatestCsv();
        if (result.ok) {
          update((state) => ({ ...state, message: result.message }));
          await this.load();
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
          update((state) => ({ ...state, message: result.message }));
          await this.load();
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
          update((state) => ({ ...state, message: result.message }));
        }
      } catch (error) {
        update((state) => ({ ...state, message: String(error) }));
      }
      await this.load();
    },

    async unreconcile(id: string) {
      try {
        const result = await paymentUnreconcile(id);
        if (result.ok) {
          update((state) => ({ ...state, message: result.message }));
        }
      } catch (error) {
        update((state) => ({ ...state, message: String(error) }));
      }
      await this.load();
    },

    openEditor(payment?: PaymentItemDto) {
      if (!payment) {
        // Create new payment with blank form
        update((state) => ({
          ...state,
          editor: createBlankForm(),
          message: null
        }));
      } else {
        // Pre-fill form from existing payment
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
      update((state) => {
        if (!state.editor) return state;
        return { ...state, loading: true, error: null };
      });

      try {
        let editor: PaymentDraftFormDto | null = null;
        update((state) => {
          editor = state.editor;
          return state;
        });

        if (!editor) {
          throw new Error("No editor state");
        }

        const result = await paymentCreateOrUpdate(editor);

        if (result.ok) {
          update((state) => ({
            ...state,
            message: result.message,
            editor: null,
            loading: false
          }));
          await this.load();
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
