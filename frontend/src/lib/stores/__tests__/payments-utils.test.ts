import { describe, expect, it } from "vitest";
import {
  filterCalendarEvents,
  formatLocalDate,
  formatLocalMonth,
  formatMoneyValue,
  parseMoneyValue,
  plusDays,
  shiftMonth
} from "../paymentsUtils";
import type { PaymentCalendarDayDto } from "../../types";

describe("paymentsUtils", () => {
  it("formats and shifts local dates without UTC conversion", () => {
    expect(formatLocalDate(new Date(2026, 4, 5))).toBe("2026-05-05");
    expect(formatLocalMonth(new Date(2026, 4, 5))).toBe("2026-05");
    expect(shiftMonth("2026-05", -1)).toBe("2026-04");
    expect(plusDays("2026-05-01", -1)).toBe("2026-04-30");
  });

  it("parses and formats Ukrainian money strings", () => {
    expect(parseMoneyValue("1 234,50 грн")).toBe(123450n);
    expect(formatMoneyValue(123450n)).toBe("1\u00a0234,50");
    expect(formatMoneyValue(-1000n)).toBe("0,00");
  });

  it("filters calendar events by kind", () => {
    const day: PaymentCalendarDayDto = {
      date: "2026-05-05",
      dayNumber: 5,
      weekdayShort: "Вт",
      inCurrentMonth: true,
      today: false,
      selected: true,
      hasOverdue: false,
      incomeTotalStr: "100,00",
      expenseTotalStr: "50,00",
      eventCount: 2,
      events: [
        {
          id: "income-1",
          kind: "schedule",
          title: "Оплата",
          subtitle: "",
          date: "2026-05-05",
          amountStr: "100,00",
          amount: "100",
          direction: "income",
          statusLabel: "",
          recurrenceLabel: "",
          counterpartyId: "",
          counterpartyName: "",
          linkKind: "",
          linkId: "",
          note: "",
          actionable: true,
          overdue: false,
          done: false
        },
        {
          id: "expense-1",
          kind: "task",
          title: "Витрата",
          subtitle: "",
          date: "2026-05-05",
          amountStr: "50,00",
          amount: "50",
          direction: "expense",
          statusLabel: "",
          recurrenceLabel: "",
          counterpartyId: "",
          counterpartyName: "",
          linkKind: "",
          linkId: "",
          note: "",
          actionable: true,
          overdue: false,
          done: false
        }
      ]
    };

    expect(filterCalendarEvents(day, "all")).toHaveLength(2);
    expect(filterCalendarEvents(day, "schedule")).toEqual([day.events[0]]);
  });
});


