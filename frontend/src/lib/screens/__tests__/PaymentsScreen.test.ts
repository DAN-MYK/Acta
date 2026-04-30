/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import PaymentsScreen from "../PaymentsScreen.svelte";
import type { PaymentDraftFormDto, PaymentsScreenDto } from "../../types";

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
    message: null as string | null
  });

  return {
    paymentsState,
    closeEditor: vi.fn(),
    importCsv: vi.fn(),
    openEditor: vi.fn(),
    openManualTemplate: vi.fn(),
    reconcile: vi.fn(),
    save: vi.fn(),
    syncBank: vi.fn(),
    unreconcile: vi.fn(),
    updateFormField: vi.fn()
  };
});

vi.mock("../../stores/payments", () => ({
  paymentsStore: {
    subscribe: mocks.paymentsState.subscribe,
    closeEditor: mocks.closeEditor,
    importCsv: mocks.importCsv,
    openEditor: mocks.openEditor,
    openManualTemplate: mocks.openManualTemplate,
    reconcile: mocks.reconcile,
    save: mocks.save,
    syncBank: mocks.syncBank,
    unreconcile: mocks.unreconcile,
    updateFormField: mocks.updateFormField
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

function buttonByText(target: HTMLElement, text: string): HTMLButtonElement {
  const button = Array.from(target.querySelectorAll("button")).find((candidate) =>
    candidate.textContent?.includes(text)
  );

  expect(button).toBeTruthy();
  return button as HTMLButtonElement;
}

describe("PaymentsScreen component", () => {
  beforeEach(() => {
    mocks.paymentsState.set({
      list: makePayments(),
      loading: false,
      error: null,
      editor: null,
      message: null
    });

    for (const fn of [
      mocks.closeEditor,
      mocks.importCsv,
      mocks.openEditor,
      mocks.openManualTemplate,
      mocks.reconcile,
      mocks.save,
      mocks.syncBank,
      mocks.unreconcile,
      mocks.updateFormField
    ]) {
      fn.mockReset();
    }
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders payment KPIs and rows", () => {
    const { component, target } = renderPayments();

    expect(target.textContent).toContain("Платежі");
    expect(target.textContent).toContain("8 450,00 грн");
    expect(target.textContent).toContain("ТОВ Ромашка");
    expect(target.textContent).toContain("ФОП Петренко");

    component.$destroy();
  });

  it("routes row and reconciliation actions through the payments store", async () => {
    const { component, target } = renderPayments();

    buttonByText(target, "ТОВ Ромашка").click();
    buttonByText(target, "Звести").click();
    buttonByText(target, "Зняти зведення").click();
    buttonByText(target, "Новий платіж").click();
    await tick();

    expect(mocks.openEditor).toHaveBeenCalledWith(makePayments().items[0]);
    expect(mocks.reconcile).toHaveBeenCalledWith("payment-1");
    expect(mocks.unreconcile).toHaveBeenCalledWith("payment-2");
    expect(mocks.openEditor).toHaveBeenCalledWith();

    component.$destroy();
  });
});
