import { describe, expect, it } from "vitest";
import {
  countEventsByKind,
  getCalendarEventDirectionLabel,
  getCalendarEventKindLabel,
  getCalendarEventTone,
  getDayAriaLabel,
  getFilteredEvents,
  getSelectedCalendarEvent,
  getVisibleEventCount,
  getWeekdayLabels
} from "../payments-calendar/payment-calendar-view-model";
import type { PaymentCalendarDayDto, PaymentCalendarEventDto } from "../../types";

const scheduleEvent: PaymentCalendarEventDto = {
  id: "schedule-1",
  kind: "schedule",
  title: "Оренда офісу",
  subtitle: "ТОВ Сервіс",
  date: "2026-05-14",
  amountStr: "12 000,00",
  amount: "12000",
  direction: "expense",
  statusLabel: "Заплановано",
  recurrenceLabel: "Щомісяця",
  counterpartyId: "counterparty-1",
  counterpartyName: "ТОВ Сервіс",
  linkKind: "schedule",
  linkId: "schedule-1",
  note: "",
  actionable: true,
  overdue: true,
  done: false
};

const taskEvent: PaymentCalendarEventDto = {
  id: "task-1",
  kind: "task",
  title: "Погодити оплату",
  subtitle: "Критичний",
  date: "2026-05-15",
  amountStr: "",
  amount: "0",
  direction: "",
  statusLabel: "В роботі",
  recurrenceLabel: "",
  counterpartyId: "",
  counterpartyName: "",
  linkKind: "task",
  linkId: "task-1",
  note: "",
  actionable: true,
  overdue: false,
  done: false
};

const selectedDay: PaymentCalendarDayDto = {
  date: "2026-05-15",
  dayNumber: 15,
  weekdayShort: "Пт",
  inCurrentMonth: true,
  today: false,
  selected: true,
  hasOverdue: true,
  incomeTotalStr: "",
  expenseTotalStr: "12 000,00",
  eventCount: 2,
  events: [scheduleEvent, taskEvent]
};

describe("payment-calendar-view-model", () => {
  it("filters events and counts visible calendar events", () => {
    const days = [selectedDay];

    expect(getFilteredEvents(selectedDay, "all")).toEqual([scheduleEvent, taskEvent]);
    expect(getFilteredEvents(selectedDay, "schedule")).toEqual([scheduleEvent]);
    expect(getVisibleEventCount(days, "task")).toBe(1);
    expect(countEventsByKind(days, "schedule")).toBe(1);
  });

  it("resolves labels, tones and aria copy for calendar events", () => {
    expect(getCalendarEventTone(scheduleEvent)).toBe("overdue");
    expect(getCalendarEventKindLabel(scheduleEvent)).toBe("Платіж");
    expect(getCalendarEventDirectionLabel(scheduleEvent)).toBe("Витрата");
    expect(getCalendarEventDirectionLabel(taskEvent)).toBe("");
    expect(getDayAriaLabel(selectedDay, "all")).toBe("2026-05-15, вибрано, 2 події");
  });

  it("selects preferred event when visible and falls back to the first visible event", () => {
    expect(getSelectedCalendarEvent([scheduleEvent, taskEvent], "task-1")).toBe(taskEvent);
    expect(getSelectedCalendarEvent([scheduleEvent, taskEvent], "missing")).toBe(scheduleEvent);
    expect(getSelectedCalendarEvent([], "task-1")).toBeNull();
  });

  it("returns weekday labels from the calendar grid or Ukrainian defaults", () => {
    expect(getWeekdayLabels([selectedDay])).toEqual(["Пт"]);
    expect(getWeekdayLabels([])).toEqual(["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Нд"]);
  });
});
