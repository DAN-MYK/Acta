import { describe, expect, it } from "vitest";
import { daysUntil, formatDate } from "../date";

describe("daysUntil", () => {
  it("returns null for empty and invalid values", () => {
    expect(daysUntil("")).toBeNull();
    expect(daysUntil("not-a-date")).toBeNull();
  });

  it("returns remaining whole days relative to the provided now value", () => {
    const now = Date.UTC(2026, 4, 5, 0, 0, 0);

    expect(daysUntil("2026-05-05", now)).toBe(0);
    expect(daysUntil("2026-05-06", now)).toBe(1);
    expect(daysUntil("2026-05-12", now)).toBe(7);
    expect(daysUntil("2026-05-04", now)).toBe(-1);
  });
});

describe("formatDate", () => {
  it("formats ISO date to Ukrainian ДД.ММ.РРРР", () => {
    expect(formatDate("2026-05-01")).toBe("01.05.2026");
    expect(formatDate("2026-04-29")).toBe("29.04.2026");
    expect(formatDate("2026-01-07")).toBe("07.01.2026");
  });

  it("returns em-dash for empty or missing value", () => {
    expect(formatDate("")).toBe("—");
    expect(formatDate(null)).toBe("—");
    expect(formatDate(undefined)).toBe("—");
  });

  it("returns em-dash for invalid date string", () => {
    expect(formatDate("not-a-date")).toBe("—");
    expect(formatDate("2026-99-99")).toBe("—");
  });
});
