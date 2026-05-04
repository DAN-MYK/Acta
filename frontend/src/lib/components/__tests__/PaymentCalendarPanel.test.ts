/**
 * @vitest-environment jsdom
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import PaymentCalendarPanel from "../PaymentCalendarPanel.svelte";
import type { PaymentCalendarMonthDto } from "../../types";

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
    list: null,
    calendar: null as PaymentCalendarMonthDto | null,
    calendarInitialLoading: false,
    calendarLoading: false,
    calendarError: null as string | null,
    calendarFilter: "all" as "all" | "schedule" | "task",
    selectedCalendarEventId: "task-1" as string | null,
    initialLoading: false,
    loading: false,
    error: null as string | null,
    editor: null,
    message: null as string | null,
    matchPreview: null,
    selectedCandidateId: null as string | null,
    manualPicker: null,
    splitDraft: null,
    activeAction: null as string | null,
    activePaymentId: null as string | null
  });

  return {
    paymentsState,
    completeSchedule: vi.fn(),
    createPaymentFromSchedule: vi.fn(),
    loadCalendar: vi.fn(),
    moveCalendarSelection: vi.fn(),
    openCalendarCounterparty: vi.fn(),
    openCalendarTask: vi.fn(),
    selectCalendarDate: vi.fn(),
    selectCalendarEvent: vi.fn(),
    setCalendarFilter: vi.fn(),
    shiftCalendarMonth: vi.fn()
  };
});

vi.mock("../../stores/payments", () => ({
  paymentsStore: {
    subscribe: mocks.paymentsState.subscribe,
    completeSchedule: mocks.completeSchedule,
    createPaymentFromSchedule: mocks.createPaymentFromSchedule,
    loadCalendar: mocks.loadCalendar,
    moveCalendarSelection: mocks.moveCalendarSelection,
    openCalendarCounterparty: mocks.openCalendarCounterparty,
    openCalendarTask: mocks.openCalendarTask,
    selectCalendarDate: mocks.selectCalendarDate,
    selectCalendarEvent: mocks.selectCalendarEvent,
    setCalendarFilter: mocks.setCalendarFilter,
    shiftCalendarMonth: mocks.shiftCalendarMonth
  }
}));

function setCalendarState(calendar: PaymentCalendarMonthDto | null, extra: Partial<{
  calendarInitialLoading: boolean;
  calendarLoading: boolean;
  calendarError: string | null;
  calendarFilter: "all" | "schedule" | "task";
  selectedCalendarEventId: string | null;
}> = {}) {
  mocks.paymentsState.set({
    list: null,
    calendar,
    calendarInitialLoading: false,
    calendarLoading: false,
    calendarError: null,
    calendarFilter: "all" as "all" | "schedule" | "task",
    selectedCalendarEventId: "task-1" as string | null,
    initialLoading: false,
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
    ...extra
  });
}

function renderCalendar() {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new PaymentCalendarPanel({ target });
  return { component, target };
}

describe("PaymentCalendarPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders month grid and opens a linked task from the details panel", async () => {
    setCalendarState({
      month: "2026-05",
      monthLabel: "Травень 2026",
      selectedDate: "2026-05-15",
      today: "2026-05-14",
      days: [
        {
          date: "2026-05-12",
          dayNumber: 12,
          weekdayShort: "Вт",
          inCurrentMonth: true,
          today: false,
          selected: false,
          hasOverdue: false,
          incomeTotalStr: "",
          expenseTotalStr: "",
          eventCount: 0,
          events: []
        },
        {
          date: "2026-05-13",
          dayNumber: 13,
          weekdayShort: "Ср",
          inCurrentMonth: true,
          today: false,
          selected: false,
          hasOverdue: false,
          incomeTotalStr: "",
          expenseTotalStr: "",
          eventCount: 0,
          events: []
        },
        {
          date: "2026-05-14",
          dayNumber: 14,
          weekdayShort: "Чт",
          inCurrentMonth: true,
          today: true,
          selected: false,
          hasOverdue: true,
          incomeTotalStr: "12 000,00",
          expenseTotalStr: "",
          eventCount: 1,
          events: [
            {
              id: "schedule-1",
              kind: "schedule",
              title: "Оренда офісу",
              subtitle: "ТОВ Сервіс",
              date: "2026-05-14",
              amountStr: "12 000,00",
              direction: "expense",
              statusLabel: "Заплановано",
              recurrenceLabel: "Щомісяця",
              counterpartyId: "counterparty-1",
              counterpartyName: "ТОВ Сервіс",
              linkKind: "schedule",
              linkId: "schedule-1",
              note: "Сплатити до обіду",
              actionable: true,
              overdue: true,
              done: false
            }
          ]
        },
        {
          date: "2026-05-15",
          dayNumber: 15,
          weekdayShort: "Пт",
          inCurrentMonth: true,
          today: false,
          selected: true,
          hasOverdue: false,
          incomeTotalStr: "",
          expenseTotalStr: "",
          eventCount: 1,
          events: [
            {
              id: "task-1",
              kind: "task",
              title: "Погодити оплату",
              subtitle: "Критичний • Рахунок №42",
              date: "2026-05-15",
              amountStr: "",
              direction: "",
              statusLabel: "В роботі",
              recurrenceLabel: "",
              counterpartyId: "",
              counterpartyName: "",
              linkKind: "task",
              linkId: "task-1",
              note: "Потрібне підтвердження директора",
              actionable: true,
              overdue: false,
              done: false
            }
          ]
        },
        {
          date: "2026-05-16",
          dayNumber: 16,
          weekdayShort: "Сб",
          inCurrentMonth: true,
          today: false,
          selected: false,
          hasOverdue: false,
          incomeTotalStr: "",
          expenseTotalStr: "",
          eventCount: 0,
          events: []
        },
        {
          date: "2026-05-17",
          dayNumber: 17,
          weekdayShort: "Нд",
          inCurrentMonth: true,
          today: false,
          selected: false,
          hasOverdue: false,
          incomeTotalStr: "",
          expenseTotalStr: "",
          eventCount: 0,
          events: []
        },
        {
          date: "2026-05-18",
          dayNumber: 18,
          weekdayShort: "Пн",
          inCurrentMonth: true,
          today: false,
          selected: false,
          hasOverdue: false,
          incomeTotalStr: "",
          expenseTotalStr: "",
          eventCount: 0,
          events: []
        }
      ]
    });

    const { component, target } = renderCalendar();
    await tick();

    expect(target.textContent ?? "").toContain("Платіжний календар");
    expect(target.textContent ?? "").toContain("Травень 2026");
    expect(target.textContent ?? "").toContain("Оренда офісу");
    expect(target.textContent ?? "").toContain("Погодити оплату");

    const openTaskButton = Array.from(target.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("Відкрити задачу")
    );
    expect(openTaskButton).toBeTruthy();
    openTaskButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));

    expect(mocks.openCalendarTask).toHaveBeenCalledWith("task-1");
    component.$destroy();
  });

  it("shows an error state when calendar loading fails", async () => {
    setCalendarState(null, {
      calendarError: "Помилка завантаження календаря"
    });

    const { component, target } = renderCalendar();
    await tick();

    expect(target.querySelector('[data-testid="payments-calendar-error"]')?.textContent).toContain(
      "Помилка завантаження календаря"
    );

    component.$destroy();
  });
});
