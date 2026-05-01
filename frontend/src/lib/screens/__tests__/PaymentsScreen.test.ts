/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import PaymentsScreen from "../PaymentsScreen.svelte";
import type {
  PaymentDraftFormDto,
  PaymentManualMatchCandidatesDto,
  PaymentMatchPreviewDto,
  PaymentsScreenDto
} from "../../types";

const mocks = vi.hoisted(() => {
  function createMockStore<T>(initialValue: T) {
    let value = initialValue;
    const subscribers = new Set<(value: T) => void>();

    return {
      subscribe(run: (value: T) => void) {
        run(value);
        subscribers.add(run);
        return () => subscribers.delete(run);
      },
      set(nextValue: T) {
        value = nextValue;
        for (const run of subscribers) {
          run(value);
        }
      }
    };
  }

  const paymentsState = createMockStore({
    list: null as PaymentsScreenDto | null,
    loading: false,
    error: null as string | null,
    editor: null as PaymentDraftFormDto | null,
    message: null as string | null,
    matchPreview: null as PaymentMatchPreviewDto | null,
    selectedCandidateId: null as string | null,
    manualPicker: null as
      | (PaymentManualMatchCandidatesDto & {
          selectedCandidateId: string | null;
        })
      | null,
    splitDraft: null as
      | {
          paymentId: string;
          paymentAmountStr: string;
          remainingAmountStr: string;
          allocations: Array<{
            documentId: string;
            documentKind: "act" | "invoice";
            title: string;
            openAmountStr: string;
            amount: string;
          }>;
        }
      | null,
    activeAction: null as string | null,
    activePaymentId: null as string | null
  });

  return {
    paymentsState,
    closeEditor: vi.fn(),
    closeManualMatchPicker: vi.fn(),
    closeMatchPreview: vi.fn(),
    confirmSplitDraft: vi.fn(),
    confirmManualPickerCandidate: vi.fn(),
    confirmPreviewAutoMatch: vi.fn(),
    confirmSelectedPreviewCandidate: vi.fn(),
    importCsv: vi.fn(),
    openEditor: vi.fn(),
    openManualMatchPicker: vi.fn(),
    openManualTemplate: vi.fn(),
    addSelectedManualPickerCandidateToSplit: vi.fn(),
    reconcile: vi.fn(),
    removeSplitAllocation: vi.fn(),
    save: vi.fn(),
    searchManualMatchCandidates: vi.fn(),
    selectManualPickerCandidate: vi.fn(),
    selectPreviewCandidate: vi.fn(),
    syncBank: vi.fn(),
    unreconcile: vi.fn(),
    updateFormField: vi.fn(),
    updateManualMatchQuery: vi.fn(),
    updateSplitAllocationAmount: vi.fn()
  };
});

vi.mock("../../stores/payments", () => ({
  paymentsStore: {
    subscribe: mocks.paymentsState.subscribe,
    closeEditor: mocks.closeEditor,
    closeManualMatchPicker: mocks.closeManualMatchPicker,
    closeMatchPreview: mocks.closeMatchPreview,
    confirmSplitDraft: mocks.confirmSplitDraft,
    confirmManualPickerCandidate: mocks.confirmManualPickerCandidate,
    confirmPreviewAutoMatch: mocks.confirmPreviewAutoMatch,
    confirmSelectedPreviewCandidate: mocks.confirmSelectedPreviewCandidate,
    importCsv: mocks.importCsv,
    openEditor: mocks.openEditor,
    openManualMatchPicker: mocks.openManualMatchPicker,
    openManualTemplate: mocks.openManualTemplate,
    addSelectedManualPickerCandidateToSplit: mocks.addSelectedManualPickerCandidateToSplit,
    reconcile: mocks.reconcile,
    removeSplitAllocation: mocks.removeSplitAllocation,
    save: mocks.save,
    searchManualMatchCandidates: mocks.searchManualMatchCandidates,
    selectManualPickerCandidate: mocks.selectManualPickerCandidate,
    selectPreviewCandidate: mocks.selectPreviewCandidate,
    syncBank: mocks.syncBank,
    unreconcile: mocks.unreconcile,
    updateFormField: mocks.updateFormField,
    updateManualMatchQuery: mocks.updateManualMatchQuery,
    updateSplitAllocationAmount: mocks.updateSplitAllocationAmount
  }
}));

function makePayments(): PaymentsScreenDto {
  return {
    items: [
      {
        id: "payment-1",
        date: "2026-04-30",
        counterpartyId: "counterparty-1",
        counterparty: "ТОВ Ромашка",
        amountStr: "8 450,00 грн",
        direction: "in",
        matchedDoc: "",
        account: "ПриватБанк"
      },
      {
        id: "payment-2",
        date: "2026-05-01",
        counterpartyId: "counterparty-2",
        counterparty: "ФОП Петренко",
        amountStr: "1 200,00 грн",
        direction: "out",
        matchedDoc: "ACT-9",
        account: "mono"
      }
    ],
    counterparties: [
      {
        id: "counterparty-1",
        name: "ТОВ Ромашка"
      }
    ],
    kpi: {
      incomingStr: "8 450,00 грн",
      outgoingStr: "1 200,00 грн",
      netStr: "7 250,00 грн",
      unmatchedStr: "1",
      incomingSub: "надходження",
      outgoingSub: "витрати",
      unmatchedCount: 1
    }
  };
}

function renderPayments() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new PaymentsScreen({ target });

  return { component, target };
}

function setPaymentsState(
  overrides: Partial<{
    list: PaymentsScreenDto | null;
    loading: boolean;
    error: string | null;
    editor: PaymentDraftFormDto | null;
    message: string | null;
    matchPreview: PaymentMatchPreviewDto | null;
    selectedCandidateId: string | null;
    manualPicker:
      | (PaymentManualMatchCandidatesDto & {
          selectedCandidateId: string | null;
        })
      | null;
    splitDraft:
      | {
          paymentId: string;
          paymentAmountStr: string;
          remainingAmountStr: string;
          allocations: Array<{
            documentId: string;
            documentKind: "act" | "invoice";
            title: string;
            openAmountStr: string;
            amount: string;
          }>;
        }
      | null;
    activeAction: string | null;
    activePaymentId: string | null;
  }> = {}
) {
  mocks.paymentsState.set({
    list: makePayments(),
    loading: false,
    error: null,
    editor: null,
    message: null,
    matchPreview: null,
    selectedCandidateId: null,
    manualPicker: null,
    splitDraft: null,
    activeAction: null,
    activePaymentId: null,
    ...overrides
  });
}

function buttonByText(target: HTMLElement, text: string): HTMLButtonElement {
  const button = Array.from(target.querySelectorAll("button")).find((candidate) =>
    candidate.textContent?.includes(text)
  );

  expect(button).toBeTruthy();
  return button as HTMLButtonElement;
}

describe("PaymentsScreen component", () => {
  beforeEach(() => {
    setPaymentsState();

    for (const fn of [
      mocks.closeEditor,
      mocks.closeManualMatchPicker,
      mocks.closeMatchPreview,
      mocks.confirmSplitDraft,
      mocks.confirmManualPickerCandidate,
      mocks.confirmPreviewAutoMatch,
      mocks.confirmSelectedPreviewCandidate,
      mocks.importCsv,
      mocks.openEditor,
      mocks.openManualMatchPicker,
      mocks.openManualTemplate,
      mocks.addSelectedManualPickerCandidateToSplit,
      mocks.reconcile,
      mocks.removeSplitAllocation,
      mocks.save,
      mocks.searchManualMatchCandidates,
      mocks.selectManualPickerCandidate,
      mocks.selectPreviewCandidate,
      mocks.syncBank,
      mocks.unreconcile,
      mocks.updateFormField,
      mocks.updateManualMatchQuery,
      mocks.updateSplitAllocationAmount
    ]) {
      fn.mockReset();
    }
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders payment workflow header and grouped reconciliation sections", () => {
    const { component, target } = renderPayments();

    expect(target.textContent).toContain("Платежі");
    expect(target.textContent).toContain("Контроль руху грошей");
    expect(target.textContent).toContain("Імпорт");
    expect(target.textContent).toContain("Звірка");
    expect(target.textContent).toContain("Ручний платіж");
    expect(target.textContent).toContain("Потребують звірки");
    expect(target.textContent).toContain("Вже зведені");
    expect(target.textContent).toContain("8 450,00 грн");
    expect(target.textContent).toContain("ТОВ Ромашка");
    expect(target.textContent).toContain("ФОП Петренко");
    expect(target.textContent).toContain("Не зведено");
    expect(target.textContent).toContain("Зв'язано з ACT-9");

    component.$destroy();
  });

  it("uses stronger reconciliation CTAs and separated row states", () => {
    const { component, target } = renderPayments();

    expect(buttonByText(target, "Імпортувати виписку").className).toContain("btn-primary");
    expect(buttonByText(target, "Запустити звірку").className).toContain("btn-secondary");
    expect(buttonByText(target, "Створити платіж").className).toContain("btn-secondary");
    expect(buttonByText(target, "Звести").className).toContain("btn-primary");
    expect(buttonByText(target, "Зняти зведення").className).toContain("btn-secondary");

    const unmatchedGroup = target.querySelector('[data-testid="payments-unmatched-group"]');
    const matchedGroup = target.querySelector('[data-testid="payments-matched-group"]');
    expect(unmatchedGroup?.textContent).toContain("ТОВ Ромашка");
    expect(unmatchedGroup?.textContent).not.toContain("ФОП Петренко");
    expect(matchedGroup?.textContent).toContain("ФОП Петренко");
    expect(matchedGroup?.textContent).not.toContain("ТОВ Ромашка");

    component.$destroy();
  });

  it("routes row and reconciliation actions through the payments store", async () => {
    const { component, target } = renderPayments();

    buttonByText(target, "ТОВ Ромашка").click();
    buttonByText(target, "Звести").click();
    buttonByText(target, "Зняти зведення").click();
    buttonByText(target, "Створити платіж").click();
    await tick();

    expect(mocks.openEditor).toHaveBeenCalledWith(makePayments().items[0]);
    expect(mocks.reconcile).toHaveBeenCalledWith("payment-1");
    expect(mocks.unreconcile).toHaveBeenCalledWith("payment-2");
    expect(mocks.openEditor).toHaveBeenCalledWith();

    component.$destroy();
  });

  it("uses canonical date control and scenario-first payment editor", () => {
    setPaymentsState({
      editor: {
        id: "",
        date: "2026-05-01",
        amount: "1000,00",
        direction: "income",
        counterpartyId: "",
        counterpartyName: "",
        bankName: "ПриватБанк",
        reference: "REF-1",
        description: "Тестовий платіж"
      },
      message: null
    });

    const { component, target } = renderPayments();
    const dateInput = Array.from(target.querySelectorAll("input")).find((input) =>
      (input as HTMLInputElement).value === "2026-05-01"
    ) as HTMLInputElement | undefined;
    const saveButton = buttonByText(target, "Зберегти");

    expect(target.textContent).toContain("Картка платежу");
    expect(target.textContent).toContain("Перевірте напрям, суму, контрагента");
    expect(target.textContent).toContain("Референс платежу");
    expect(target.textContent).toContain("Пов'язаний документ");
    expect(dateInput).toBeTruthy();
    expect(dateInput?.type).toBe("date");
    expect(saveButton.className).toContain("btn-primary");

    component.$destroy();
  });

  it("shows visible import loading and empty states for payment review flow", () => {
    setPaymentsState({
      list: {
        items: [],
        counterparties: [],
        kpi: {
          incomingStr: "0,00 грн",
          outgoingStr: "0,00 грн",
          netStr: "0,00 грн",
          unmatchedStr: "0",
          incomingSub: "надходження",
          outgoingSub: "витрати",
          unmatchedCount: 0
        }
      },
      loading: true,
      activeAction: "import"
    });

    const { component, target } = renderPayments();
    const importButton = buttonByText(target, "Імпортуємо виписку");

    expect(target.textContent).toContain("Імпорт триває");
    expect(target.textContent).toContain("Імпортуємо виписку");
    expect(target.textContent).toContain("Ще немає жодного платежу");
    expect(target.textContent).toContain("Імпортуйте виписку");
    expect(importButton.disabled).toBe(true);

    component.$destroy();
  });

  it("shows reconciliation preview states with clear next step and errors", () => {
    setPaymentsState({
      loading: true,
      activeAction: "reconcile",
      activePaymentId: "payment-1",
      matchPreview: {
        paymentId: "payment-1",
        isReconciled: false,
        decisionKind: "none",
        candidates: [],
        autoMatch: null
      },
      error: "Не вдалося імпортувати виписку"
    });

    const { component, target } = renderPayments();

    expect(target.textContent).toContain("Готуємо preview звірки");
    expect(target.textContent).toContain("Автоматична звірка не знайшла");
    expect(target.textContent).toContain("Не вдалося імпортувати виписку");
    expect(buttonByText(target, "Зводимо").disabled).toBe(true);

    component.$destroy();
  });

  it("shows manual picker CTA for no-match preview and routes manual picker actions", async () => {
    setPaymentsState({
      matchPreview: {
        paymentId: "payment-1",
        isReconciled: false,
        decisionKind: "none",
        candidates: [],
        autoMatch: null
      },
      manualPicker: {
        paymentId: "payment-1",
        query: "ACT",
        candidates: [
          {
            documentId: "act-1",
            documentKind: "act",
            title: "Акт ACT-001",
            openAmountStr: "8 450,00 грн",
            totalScore: 98,
            sameIban: true,
            referenceHit: true,
            textHits: 2,
            daysDistance: 0
          }
        ],
        selectedCandidateId: "act-1"
      }
    });

    const { component, target } = renderPayments();
    const searchInput = target.querySelector('[data-testid="payments-manual-picker"] input') as HTMLInputElement | null;

    expect(target.querySelector('[data-testid="payments-manual-picker"]')).toBeTruthy();
    expect(searchInput).toBeTruthy();

    searchInput!.value = "INV-42";
    searchInput!.dispatchEvent(new Event("input", { bubbles: true }));
    buttonByText(target, "Ручний пошук документа").click();
    buttonByText(target, "Оновити пошук").click();
    buttonByText(target, "Підтвердити вибраний документ").click();
    buttonByText(target, "Вибрано").click();
    await tick();

    expect(mocks.updateManualMatchQuery).toHaveBeenCalledWith("INV-42");
    expect(mocks.openManualMatchPicker).toHaveBeenCalledWith("payment-1");
    expect(mocks.searchManualMatchCandidates).toHaveBeenCalled();
    expect(mocks.confirmManualPickerCandidate).toHaveBeenCalled();
    expect(mocks.selectManualPickerCandidate).toHaveBeenCalledWith("act-1");

    component.$destroy();
  });

  it("disables manual confirmation when there is no selected candidate", () => {
    setPaymentsState({
      matchPreview: {
        paymentId: "payment-1",
        isReconciled: false,
        decisionKind: "none",
        candidates: [],
        autoMatch: null
      },
      manualPicker: {
        paymentId: "payment-1",
        query: "ACT",
        candidates: [
          {
            documentId: "act-1",
            documentKind: "act",
            title: "Акт ACT-001",
            openAmountStr: "8 450,00 грн",
            totalScore: 98,
            sameIban: true,
            referenceHit: true,
            textHits: 2,
            daysDistance: 0
          }
        ],
        selectedCandidateId: null
      }
    });

    const { component, target } = renderPayments();

    expect(buttonByText(target, "Підтвердити вибраний документ").disabled).toBe(true);

    component.$destroy();
  });

  it("disables manual confirmation when there are no candidates", () => {
    setPaymentsState({
      matchPreview: {
        paymentId: "payment-1",
        isReconciled: false,
        decisionKind: "none",
        candidates: [],
        autoMatch: null
      },
      manualPicker: {
        paymentId: "payment-1",
        query: "missing",
        candidates: [],
        selectedCandidateId: null
      }
    });

    const { component, target } = renderPayments();

    expect(target.textContent).toContain("За цим запитом кандидатів поки немає.");
    expect(buttonByText(target, "Підтвердити вибраний документ").disabled).toBe(true);

    component.$destroy();
  });
});
