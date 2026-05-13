import { formatMinorMoney, parseMoneyToMinor } from "../money";
import type {
  PaymentCalendarDayDto,
  PaymentCalendarEventDto,
  PaymentCalendarFilterKind
} from "../types";

export { parseMoneyToMinor, formatMinorMoney };

export function formatLocalDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function formatLocalMonth(date: Date): string {
  return formatLocalDate(date).slice(0, 7);
}

export function shiftMonth(month: string, delta: number): string {
  const [yearPart, monthPart] = month.split("-");
  const year = Number.parseInt(yearPart ?? "", 10);
  const monthIndex = Number.parseInt(monthPart ?? "", 10);
  const anchor = new Date(year, Math.max(monthIndex - 1, 0), 1);
  anchor.setMonth(anchor.getMonth() + delta);
  return formatLocalMonth(anchor);
}

export function plusDays(dateValue: string, delta: number): string {
  const [yearPart, monthPart, dayPart] = dateValue.split("-");
  const anchor = new Date(
    Number.parseInt(yearPart ?? "", 10),
    Math.max(Number.parseInt(monthPart ?? "", 10) - 1, 0),
    Number.parseInt(dayPart ?? "", 10)
  );
  anchor.setDate(anchor.getDate() + delta);
  return formatLocalDate(anchor);
}

export function parseMoneyValue(value: string): bigint {
  return parseMoneyToMinor(value) ?? 0n;
}

export function formatMoneyValue(minor: bigint): string {
  return formatMinorMoney(minor < 0n ? 0n : minor);
}

export function filterCalendarEvents(
  day: PaymentCalendarDayDto | null | undefined,
  filter: PaymentCalendarFilterKind
): PaymentCalendarEventDto[] {
  if (!day) {
    return [];
  }

  if (filter === "all") {
    return day.events;
  }

  return day.events.filter((event) => event.kind === filter);
}
