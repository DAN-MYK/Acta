import {
  CALENDAR_EVENT_KIND_LABELS,
  formatCalendarDayAriaLabel,
  getCalendarEventDirectionLabel as getDirectionLabel
} from "../../config/ui";
import type {
  PaymentCalendarDayDto,
  PaymentCalendarEventDto,
  PaymentCalendarEventKind,
  PaymentCalendarFilterKind
} from "../../types";

const DEFAULT_WEEKDAY_LABELS = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Нд"];

export function getFilteredEvents(
  day: PaymentCalendarDayDto,
  filter: PaymentCalendarFilterKind
): PaymentCalendarEventDto[] {
  if (filter === "all") {
    return day.events;
  }

  return day.events.filter((event) => event.kind === filter);
}

export function countEventsByKind(
  days: PaymentCalendarDayDto[],
  kind: PaymentCalendarEventKind
): number {
  return days.reduce(
    (sum, day) => sum + day.events.filter((event) => event.kind === kind).length,
    0
  );
}

export function getVisibleEventCount(
  days: PaymentCalendarDayDto[],
  filter: PaymentCalendarFilterKind
): number {
  return days.reduce((sum, day) => sum + getFilteredEvents(day, filter).length, 0);
}

export function getCalendarEventTone(event: PaymentCalendarEventDto) {
  if (event.done) {
    return "done";
  }
  if (event.overdue) {
    return "overdue";
  }
  return event.kind;
}

export function getCalendarEventKindLabel(event: PaymentCalendarEventDto): string {
  return CALENDAR_EVENT_KIND_LABELS[event.kind];
}

export function getCalendarEventDirectionLabel(event: PaymentCalendarEventDto): string {
  if (event.kind !== "schedule") {
    return "";
  }

  return getDirectionLabel(event.direction);
}

export function getDayAriaLabel(
  day: PaymentCalendarDayDto,
  filter: PaymentCalendarFilterKind
): string {
  return formatCalendarDayAriaLabel({
    date: day.date,
    eventCount: getFilteredEvents(day, filter).length,
    today: day.today,
    selected: day.selected
  });
}

export function getSelectedCalendarEvent(
  selectedEvents: PaymentCalendarEventDto[],
  selectedEventId: string | null
): PaymentCalendarEventDto | null {
  return (
    selectedEvents.find((event) => event.id === selectedEventId) ??
    selectedEvents[0] ??
    null
  );
}

export function getWeekdayLabels(days: PaymentCalendarDayDto[]): string[] {
  const labels = days.slice(0, 7).map((day) => day.weekdayShort);
  return labels.length > 0 ? labels : DEFAULT_WEEKDAY_LABELS;
}
