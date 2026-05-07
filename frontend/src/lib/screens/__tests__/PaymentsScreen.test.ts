/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import PaymentsScreen from "../PaymentsScreen.svelte";
import type {
  PaymentCalendarMonthDto,
  PaymentDraftFormDto,
  PaymentImportPreviewDto,
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
    calendar: null as PaymentCalendarMonthDto | null,
    calendarInitialLoading: false,
    calendarLoading: false,
    calendarError: null as string | null,
    calendarFilter: "all" as "all" | "schedule" | "task",
    selectedCalendarEventId: null as string | null,
    initialLoading: false,
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
    importPreview: null as PaymentImportPreviewDto | null,
    importPreviewStale: false,
    activeAction: null as string | null,
    activePaymentId: null as string | null
  });

  return {
    paymentsState,
    cancelImportPreview: vi.fn(),
    closeEditor: vi.fn(() => ({ ok: true })),
    closeManualMatchPicker: vi.fn(),
    closeMatchPreview: vi.fn(),
    commitImportPreview: vi.fn(),
    confirmSplitDraft: vi.fn(),
    completeSchedule: vi.fn(),
    createPaymentFromSchedule: vi.fn(),
    confirmManualPickerCandidate: vi.fn(),
    confirmPreviewAutoMatch: vi.fn(),
    confirmSelectedPreviewCandidate: vi.fn(),
    refreshImportPreview: vi.fn(),
    importCsv: vi.fn(),
    pickAndPreviewImport: vi.fn(),
    loadCalendar: vi.fn(),
    moveCalendarSelection: vi.fn(),
    openEditor: vi.fn(),
    openCalendarCounterparty: vi.fn(),
    openCalendarTask: vi.fn(),
    openManualMatchPicker: vi.fn(),
    openManualTemplate: vi.fn(),
    addSelectedManualPickerCandidateToSplit: vi.fn(),
    reconcile: vi.fn(),
    removeSplitAllocation: vi.fn(),
    save: vi.fn(),
    searchManualMatchCandidates: vi.fn(),
    selectCalendarDate: vi.fn(),
    selectCalendarEvent: vi.fn(),
    selectManualPickerCandidate: vi.fn(),
    selectPreviewCandidate: vi.fn(),
    setCalendarFilter: vi.fn(),
    shiftCalendarMonth: vi.fn(),
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
    cancelImportPreview: mocks.cancelImportPreview,
    closeEditor: mocks.closeEditor,
    closeManualMatchPicker: mocks.closeManualMatchPicker,
    closeMatchPreview: mocks.closeMatchPreview,
    commitImportPreview: mocks.commitImportPreview,
    confirmSplitDraft: mocks.confirmSplitDraft,
    completeSchedule: mocks.completeSchedule,
    createPaymentFromSchedule: mocks.createPaymentFromSchedule,
    confirmManualPickerCandidate: mocks.confirmManualPickerCandidate,
    confirmPreviewAutoMatch: mocks.confirmPreviewAutoMatch,
    confirmSelectedPreviewCandidate: mocks.confirmSelectedPreviewCandidate,
    refreshImportPreview: mocks.refreshImportPreview,
    importCsv: mocks.importCsv,
    pickAndPreviewImport: mocks.pickAndPreviewImport,
    loadCalendar: mocks.loadCalendar,
    moveCalendarSelection: mocks.moveCalendarSelection,
    openEditor: mocks.openEditor,
    openCalendarCounterparty: mocks.openCalendarCounterparty,
    openCalendarTask: mocks.openCalendarTask,
    openManualMatchPicker: mocks.openManualMatchPicker,
    openManualTemplate: mocks.openManualTemplate,
    addSelectedManualPickerCandidateToSplit: mocks.addSelectedManualPickerCandidateToSplit,
    reconcile: mocks.reconcile,
    removeSplitAllocation: mocks.removeSplitAllocation,
    save: mocks.save,
    searchManualMatchCandidates: mocks.searchManualMatchCandidates,
    selectCalendarDate: mocks.selectCalendarDate,
    selectCalendarEvent: mocks.selectCalendarEvent,
    selectManualPickerCandidate: mocks.selectManualPickerCandidate,
    selectPreviewCandidate: mocks.selectPreviewCandidate,
    setCalendarFilter: mocks.setCalendarFilter,
    shiftCalendarMonth: mocks.shiftCalendarMonth,
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
    calendar: PaymentCalendarMonthDto | null;
    calendarInitialLoading: boolean;
    calendarLoading: boolean;
    calendarError: string | null;
    calendarFilter: "all" | "schedule" | "task";
    selectedCalendarEventId: string | null;
    initialLoading: boolean;
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
    importPreview: PaymentImportPreviewDto | null;
    importPreviewStale: boolean;
    activeAction: string | null;
    activePaymentId: string | null;
  }> = {}
) {
  mocks.paymentsState.set({
    list: makePayments(),
    calendar: null,
    calendarInitialLoading: false,
    calendarLoading: false,
    calendarError: null,
    calendarFilter: "all",
    selectedCalendarEventId: null,
    initialLoading: false,
    loading: false,
    error: null,
    editor: null,
    message: null,
    matchPreview: null,
    selectedCandidateId: null,
    manualPicker: null,
    splitDraft: null,
    importPreview: null,
    importPreviewStale: false,
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
      mocks.cancelImportPreview,
      mocks.closeEditor,
      mocks.closeManualMatchPicker,
      mocks.closeMatchPreview,
      mocks.commitImportPreview,
      mocks.confirmSplitDraft,
      mocks.confirmManualPickerCandidate,
      mocks.confirmPreviewAutoMatch,
      mocks.confirmSelectedPreviewCandidate,
      mocks.refreshImportPreview,
      mocks.importCsv,
      mocks.openEditor,
      mocks.openManualMatchPicker,
      mocks.openManualTemplate,
      mocks.pickAndPreviewImport,
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
    mocks.closeEditor.mockReturnValue({ ok: true });
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders payment workflow header and grouped reconciliation sections", () => {
    const { component, target } = renderPayments();

    expect(target.textContent).toContain("Платежі");
    expect(target.textContent).toContain("Імпортувати виписку");
    expect(target.textContent).toContain("Запустити звірку");
    expect(target.textContent).toContain("Створити платіж");
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

  it("groups compact toolbar actions into primary and utility zones", () => {
    const { component, target } = renderPayments();
    const mainToolbar = target.querySelector('[data-testid="payments-toolbar-main"]');
    const utilityToolbar = target.querySelector('[data-testid="payments-toolbar-utility"]');
    const utilityButtons = Array.from(utilityToolbar?.querySelectorAll("button") ?? []).map((button) =>
      button.textContent?.trim()
    );

    expect(mainToolbar).toBeTruthy();
    expect(utilityToolbar).toBeTruthy();
    expect(buttonByText(target, "Імпортувати виписку").className).toContain("payments-toolbar-primary-action");
    expect(utilityButtons).toEqual(["Імпорт з storage", "Оновити з банку", "Шаблон CSV"]);

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
    const importButton = buttonByText(target, "Імпортуємо...");

    expect(target.textContent).toContain("Імпорт триває");
    expect(target.textContent).toContain("Ще немає жодного платежу");
    expect(target.textContent).toContain("Імпортуйте виписку");
    expect(importButton.disabled).toBe(true);

    component.$destroy();
  });

  it("shows import preview section and routes commit and cancel actions", async () => {
    setPaymentsState({
      importPreview: {
        ok: true,
        message: "Знайдено 3 рядки. Буде створено 2, пропущено 1",
        path: "/tmp/bank.csv",
        bankName: "ПриватБанк",
        parsed: 3,
        willCreate: 2,
        willSkip: 1,
        conflicts: 0,
        fileSize: 8192,
        fileMtimeSecs: 1746355200,
        fileHash: "0123456789abcdef",
        rows: [
          { action: "create", bankRef: "REF-1", description: "Перший платіж", note: "create: bank_ref відсутній у БД" },
          { action: "create", bankRef: "REF-2", description: "Другий платіж", note: "create: bank_ref відсутній у БД" },
          { action: "skip", bankRef: "REF-3", description: "Дубль", note: "skip: знайдено existing row за bank_ref" }
        ]
      }
    });

    const { component, target } = renderPayments();
    await tick();

    const preview = target.querySelector('[data-testid="payments-import-preview"]');
    expect(preview).toBeTruthy();
    expect(preview?.textContent).toContain("ПриватБанк");
    expect(preview?.textContent).toContain("Знайдено 3 рядки");
    expect(preview?.textContent).toContain("Розпізнано рядків");
    expect(preview?.textContent).toContain("Буде створено");
    expect(preview?.textContent).toContain("Перший платіж");
    expect(preview?.textContent).toContain("Дубль");

    const commitBtn = buttonByText(target, "Імпортувати 2 платежі(ів)");
    expect(commitBtn.className).toContain("btn-primary");
    expect(commitBtn.disabled).toBe(false);

    const cancelBtn = buttonByText(target, "Скасувати");
    commitBtn.click();
    cancelBtn.click();
    await tick();

    expect(mocks.commitImportPreview).toHaveBeenCalledTimes(1);
    expect(mocks.cancelImportPreview).toHaveBeenCalledTimes(1);

    component.$destroy();
  });

  it("disables import preview commit when willCreate is 0", () => {
    setPaymentsState({
      importPreview: {
        ok: true,
        message: "Усі рядки вже імпортовано раніше",
        path: "/tmp/old.csv",
        bankName: "Укргазбанк",
        parsed: 2,
        willCreate: 0,
        willSkip: 2,
        conflicts: 0,
        fileSize: 4096,
        fileMtimeSecs: 1746355200,
        fileHash: "fedcba9876543210",
        rows: [
          { action: "skip", bankRef: "REF-X", description: "Дубль 1", note: "skip: знайдено existing row за bank_ref" },
          { action: "skip", bankRef: "REF-Y", description: "Дубль 2", note: "skip: знайдено existing row за bank_ref" }
        ]
      }
    });

    const { component, target } = renderPayments();

    expect(buttonByText(target, "Немає нових платежів").disabled).toBe(true);

    component.$destroy();
  });

  it("shows stale import preview CTA and routes reread action", async () => {
    setPaymentsState({
      importPreview: {
        ok: true,
        message: "Файл потребує повторного preview",
        path: "/tmp/stale.csv",
        bankName: "ПриватБанк",
        parsed: 2,
        willCreate: 1,
        willSkip: 1,
        conflicts: 0,
        fileSize: 4096,
        fileMtimeSecs: 1746355200,
        fileHash: "stale-hash",
        rows: [
          { action: "create", bankRef: "REF-1", description: "Платіж", note: "create" }
        ]
      },
      importPreviewStale: true
    });

    const { component, target } = renderPayments();

    expect(target.textContent).toContain("Файл виписки змінився");
    buttonByText(target, "Перечитати файл").click();
    await tick();

    expect(mocks.refreshImportPreview).toHaveBeenCalledTimes(1);

    component.$destroy();
  });

  it("keeps chrome visible and skeletonizes payment lists during initial loading", () => {
    setPaymentsState({
      list: null,
      initialLoading: true,
      loading: false,
      message: null,
      error: null
    });

    const { component, target } = renderPayments();

    expect(target.textContent).toContain("Імпортувати виписку");
    expect(target.textContent).toContain("Запустити звірку");
    expect(target.textContent).toContain("Створити платіж");
    expect(target.querySelector('[data-testid="payments-kpis"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="payments-kpis"] [data-testid="skeleton-card-grid"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="payments-kpis"] .task-kpi-card')).toBeNull();
    expect(target.querySelectorAll('[data-testid="skeleton-row-item"]')).toHaveLength(6);
    expect(target.textContent).not.toContain("Ще немає жодного платежу");
    expect(target.textContent).not.toContain("Ще немає зведених платежів");
    expect(target.textContent).not.toContain("0,00");

    component.$destroy();
  });

  it("does not show skeletons during import loading after initial load", () => {
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
      initialLoading: false,
      loading: true,
      activeAction: "import"
    });

    const { component, target } = renderPayments();

    expect(target.querySelector('[data-testid="payments-flow-banner"]')).toBeTruthy();
    expect(target.textContent).toContain("Імпорт триває");
    expect(target.textContent).toContain("Імпортуємо виписку");
    expect(target.querySelector('[data-testid="skeleton-row-item"]')).toBeNull();

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

  it("marks the main panel inert while the editor drawer is open", () => {
    setPaymentsState({
      editor: {
        id: "payment-1",
        date: "2026-05-01",
        amount: "1000,00",
        direction: "income",
        counterpartyId: "",
        counterpartyName: "",
        bankName: "ПриватБанк",
        reference: "REF-1",
        description: "Тестовий платіж"
      }
    });

    const { component, target } = renderPayments();

    const panel = target.querySelector('[data-testid="payments-screen"]') as HTMLElement | null;
    expect(panel?.inert).toBe(true);
    expect(panel?.getAttribute("aria-hidden")).toBe("true");

    component.$destroy();
  });

  it("shows inline dirty banner before closing a dirty editor", async () => {
    mocks.closeEditor.mockReturnValue({ ok: false, reason: "dirty" } as any);
    setPaymentsState({
      editor: {
        id: "payment-1",
        date: "2026-05-01",
        amount: "1000,00",
        direction: "income",
        counterpartyId: "",
        counterpartyName: "",
        bankName: "ПриватБанк",
        reference: "REF-1",
        description: "Тестовий платіж"
      }
    });

    const { component, target } = renderPayments();

    buttonByText(target, "Закрити").click();
    await tick();

    expect(target.querySelector('[data-testid="payments-dirty-banner"]')).toBeTruthy();
    expect(target.textContent).toContain("У вас є незбережені зміни");
    expect(target.textContent).toContain("Скасувати їх і закрити форму?");
    expect(target.textContent).toContain("Залишитися");
    expect(target.textContent).toContain("Так, закрити");
    expect(mocks.closeEditor).toHaveBeenCalledWith(false);

    buttonByText(target, "Так, закрити").click();
    await tick();

    expect(mocks.closeEditor).toHaveBeenCalledWith(true);

    component.$destroy();
  });

  it("shows the dirty banner on Escape before closing the editor", async () => {
    mocks.closeEditor.mockReturnValue({ ok: false, reason: "dirty" } as any);
    setPaymentsState({
      editor: {
        id: "payment-1",
        date: "2026-05-01",
        amount: "1000,00",
        direction: "income",
        counterpartyId: "",
        counterpartyName: "",
        bankName: "ПриватБанк",
        reference: "REF-1",
        description: "Тестовий платіж"
      }
    });

    const { component, target } = renderPayments();

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true }));
    await tick();

    expect(target.querySelector('[data-testid="payments-dirty-banner"]')).toBeTruthy();
    expect(mocks.closeEditor).toHaveBeenCalledWith(false);

    component.$destroy();
  });

  it("shows the dirty banner on backdrop click before closing the editor", async () => {
    mocks.closeEditor.mockReturnValue({ ok: false, reason: "dirty" } as any);
    setPaymentsState({
      editor: {
        id: "payment-1",
        date: "2026-05-01",
        amount: "1000,00",
        direction: "income",
        counterpartyId: "",
        counterpartyName: "",
        bankName: "ПриватБанк",
        reference: "REF-1",
        description: "Тестовий платіж"
      }
    });

    const { component, target } = renderPayments();

    (target.querySelector('[data-testid="payments-editor-backdrop"]') as HTMLButtonElement).click();
    await tick();

    expect(target.querySelector('[data-testid="payments-dirty-banner"]')).toBeTruthy();
    expect(mocks.closeEditor).toHaveBeenCalledWith(false);

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

  it("renders split-draft strings without mojibake", async () => {
    setPaymentsState({
      matchPreview: {
        paymentId: "payment-1",
        isReconciled: false,
        decisionKind: "none",
        candidates: [],
        autoMatch: null
      },
      splitDraft: {
        paymentId: "payment-1",
        paymentAmountStr: "100,00 грн",
        remainingAmountStr: "20,00 грн",
        allocations: []
      }
    });

    const { component, target } = renderPayments();
    await tick();

    const text = target.textContent ?? "";

    expect(text).toContain("Чернетка розподілу");
    expect(text).toContain("Сума платежу");
    expect(text).toContain("Залишок");
    expect(text).toContain("Додайте документи з manual picker");
    expect(text).not.toContain("вЂў");

    component.$destroy();
  });

  it("renders split-allocation controls without mojibake", async () => {
    setPaymentsState({
      matchPreview: {
        paymentId: "payment-1",
        isReconciled: false,
        decisionKind: "none",
        candidates: [],
        autoMatch: null
      },
      splitDraft: {
        paymentId: "payment-1",
        paymentAmountStr: "100,00 грн",
        remainingAmountStr: "20,00 грн",
        allocations: [
          {
            documentId: "invoice-1",
            documentKind: "invoice",
            title: "INV-001",
            openAmountStr: "20,00 грн",
            amount: "20,00"
          }
        ]
      }
    });

    const { component, target } = renderPayments();
    await tick();

    const text = target.textContent ?? "";

    expect(text).toContain("Залишок документа");
    expect(text).toContain("Сума");
    expect(text).toContain("Прибрати");
    expect(text).toContain("Підтвердити розподіл");

    component.$destroy();
  });

  it("renders manual-picker split button without mojibake", async () => {
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
        query: "",
        candidates: [],
        selectedCandidateId: null
      }
    });

    const { component, target } = renderPayments();
    await tick();

    expect(target.textContent ?? "").toContain("Додати до розподілу");

    component.$destroy();
  });

  it("renders split preview with recommended candidates", async () => {
    setPaymentsState({
      matchPreview: {
        paymentId: "payment-1",
        isReconciled: false,
        decisionKind: "split",
        candidates: [
          {
            documentId: "invoice-1",
            documentKind: "invoice",
            title: "INV-001",
            openAmountStr: "4 000,00 грн",
            totalScore: 88,
            sameIban: true,
            referenceHit: true,
            textHits: 2,
            daysDistance: 1
          },
          {
            documentId: "act-9",
            documentKind: "act",
            title: "ACT-009",
            openAmountStr: "4 450,00 грн",
            totalScore: 86,
            sameIban: true,
            referenceHit: true,
            textHits: 2,
            daysDistance: 0
          }
        ],
        autoMatch: null
      },
      splitDraft: {
        paymentId: "payment-1",
        paymentAmountStr: "8 450,00 грн",
        remainingAmountStr: "0,00 грн",
        allocations: [
          {
            documentId: "invoice-1",
            documentKind: "invoice",
            title: "INV-001",
            openAmountStr: "4 000,00 грн",
            amount: "4 000,00"
          },
          {
            documentId: "act-9",
            documentKind: "act",
            title: "ACT-009",
            openAmountStr: "4 450,00 грн",
            amount: "4 450,00"
          }
        ]
      }
    });

    const { component, target } = renderPayments();
    await tick();

    const text = target.textContent ?? "";
    expect(text).toContain("Рекомендований розподіл платежу");
    expect(text).toContain("INV-001");
    expect(text).toContain("ACT-009");
    expect(text).toContain("Рахунок");
    expect(text).toContain("Акт");
    expect(text).toContain("Рекомендація для розподілу");

    component.$destroy();
  });

  it("describes why manual confirmation is unavailable", () => {
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
    const confirmButton = buttonByText(target, "Підтвердити вибраний документ");
    const descriptionId = confirmButton.getAttribute("aria-describedby");

    expect(descriptionId).toBeTruthy();
    expect(target.querySelector(`#${descriptionId}`)?.textContent).toContain("кандидат");

    component.$destroy();
  });

  it("shows bank tab content by default and hides calendar panel", () => {
    const { component, target } = renderPayments();

    expect(target.querySelector('[data-testid="payments-unmatched-group"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="payments-calendar"]')).toBeNull();

    component.$destroy();
  });

  it("switches to calendar tab on click and hides bank content", async () => {
    const { component, target } = renderPayments();

    buttonByText(target, "Платіжний календар").click();
    await tick();

    expect(target.querySelector('[data-testid="payments-unmatched-group"]')).toBeNull();
    expect(target.querySelector('[data-testid="payments-calendar"]')).toBeTruthy();

    component.$destroy();
  });

  it("switches back to bank tab from calendar tab", async () => {
    const { component, target } = renderPayments();

    buttonByText(target, "Платіжний календар").click();
    await tick();
    buttonByText(target, "Банк").click();
    await tick();

    expect(target.querySelector('[data-testid="payments-unmatched-group"]')).toBeTruthy();
    expect(target.querySelector('[data-testid="payments-calendar"]')).toBeNull();

    component.$destroy();
  });
});
