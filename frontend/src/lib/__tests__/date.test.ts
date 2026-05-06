import { describe, expect, it } from "vitest";
import { daysUntil } from "../date";

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
