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

function setPaymentsState(overrides: Partial<{
  list: PaymentsScreenDto | null;
  loading: boolean;
  error: string | null;
  editor: PaymentDraftFormDto | null;
  message: string | null;
}> = {}) {
  mocks.paymentsState.set({
    list: makePayments(),
    loading: false,
    error: null,
    editor: null,
    message: null,
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

  it("renders payment KPIs, scenario header and reconciliation states", () => {
    const { component, target } = renderPayments();

    expect(target.textContent).toContain("Платежі");
    expect(target.textContent).toContain("Контроль руху грошей");
    expect(target.textContent).toContain("Імпорт");
    expect(target.textContent).toContain("Звірка");
    expect(target.textContent).toContain("Ручний платіж");
    expect(target.textContent).toContain("8 450,00 грн");
    expect(target.textContent).toContain("ТОВ Ромашка");
    expect(target.textContent).toContain("ФОП Петренко");
    expect(target.textContent).toContain("Не зведено");
    expect(target.textContent).toContain("Зв'язано з ACT-9");

    component.$destroy();
  });

  it("uses canonical payment action hierarchy and row states", () => {
    const { component, target } = renderPayments();

    expect(buttonByText(target, "Імпортувати виписку").className).toContain("btn-secondary");
    expect(buttonByText(target, "Оновити з банку").className).toContain("btn-ghost");
    expect(buttonByText(target, "Шаблон CSV").className).toContain("btn-ghost");
    expect(buttonByText(target, "Створити платіж").className).toContain("btn-primary");
    expect(buttonByText(target, "Звірити платіж").className).toContain("btn-secondary");
    expect(buttonByText(target, "Зняти звірку").className).toContain("btn-ghost");

    const rows = target.querySelectorAll(".payment-row");
    expect(rows).toHaveLength(2);
    expect(rows[0]?.className).toContain("payment-row-unmatched");
    expect(rows[1]?.className).toContain("payment-row-matched");

    component.$destroy();
  });

  it("routes row and reconciliation actions through the payments store", async () => {
    const { component, target } = renderPayments();

    buttonByText(target, "ТОВ Ромашка").click();
    buttonByText(target, "Звірити платіж").click();
    buttonByText(target, "Зняти звірку").click();
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
    expect(target.textContent).toContain("Пов'язаний документ");
    expect(dateInput).toBeTruthy();
    expect(dateInput?.type).toBe("date");
    expect(saveButton.className).toContain("btn-primary");

    component.$destroy();
  });

  it("shows visible loading and empty states for payment review flow", () => {
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
      loading: true
    });

    const { component, target } = renderPayments();

    expect(target.textContent).toContain("Оновлюємо платежі");
    expect(target.textContent).toContain("Ще немає жодного платежу");
    expect(target.textContent).toContain("Імпортуйте виписку");

    component.$destroy();
  });
});
