import { describe, expect, it } from "vitest";
import {
  addMinor,
  compareMinor,
  formatMinorMoney,
  parseMoneyToMinor,
  subMinor
} from "../money";

describe("money — parseMoneyToMinor", () => {
  it("parses Ukrainian decimal format with NBSP thousand separator", () => {
    expect(parseMoneyToMinor("1 234,56")).toBe(123456n);
    expect(parseMoneyToMinor("1 234,56")).toBe(123456n);
    expect(parseMoneyToMinor("1 234,56 грн")).toBe(123456n);
  });

  it("accepts both comma and dot as decimal separator", () => {
    expect(parseMoneyToMinor("1234,56")).toBe(123456n);
    expect(parseMoneyToMinor("1234.56")).toBe(123456n);
  });

  it("handles whole numbers and zero scale", () => {
    expect(parseMoneyToMinor("0")).toBe(0n);
    expect(parseMoneyToMinor("0,00")).toBe(0n);
    expect(parseMoneyToMinor("100")).toBe(10000n);
    expect(parseMoneyToMinor("100,5")).toBe(10050n);
  });

  it("rounds fractional part > 2 digits half-up", () => {
    expect(parseMoneyToMinor("1,005")).toBe(101n);
    expect(parseMoneyToMinor("1,004")).toBe(100n);
    expect(parseMoneyToMinor("1,999")).toBe(200n);
  });

  it("preserves negative sign", () => {
    expect(parseMoneyToMinor("-5")).toBe(-500n);
    expect(parseMoneyToMinor("-5,00")).toBe(-500n);
    expect(parseMoneyToMinor("-1 000,01")).toBe(-100001n);
  });

  it("returns null for invalid inputs", () => {
    expect(parseMoneyToMinor("")).toBeNull();
    expect(parseMoneyToMinor("   ")).toBeNull();
    expect(parseMoneyToMinor("abc")).toBeNull();
    expect(parseMoneyToMinor("NaN")).toBeNull();
    expect(parseMoneyToMinor("Infinity")).toBeNull();
    expect(parseMoneyToMinor("-Infinity")).toBeNull();
    expect(parseMoneyToMinor("1e21")).toBeNull();
    expect(parseMoneyToMinor("1.234,56")).toBeNull();
    expect(parseMoneyToMinor("1,234.56")).toBeNull();
    expect(parseMoneyToMinor("--5")).toBeNull();
    expect(parseMoneyToMinor("5..0")).toBeNull();
  });

  it("handles very large values without precision loss", () => {
    expect(parseMoneyToMinor("999999999999999,99")).toBe(99999999999999999n);
  });
});

describe("money — formatMinorMoney", () => {
  it("formats small amounts with NBSP thousand separator", () => {
    expect(formatMinorMoney(0n)).toBe("0,00");
    expect(formatMinorMoney(50n)).toBe("0,50");
    expect(formatMinorMoney(123456n)).toBe("1 234,56");
  });

  it("formats negative amounts", () => {
    expect(formatMinorMoney(-1n)).toBe("-0,01");
    expect(formatMinorMoney(-123456n)).toBe("-1 234,56");
  });

  it("formats values across thousand boundaries", () => {
    expect(formatMinorMoney(100000n)).toBe("1 000,00");
    expect(formatMinorMoney(100000000n)).toBe("1 000 000,00");
  });
});

describe("money — addMinor / subMinor / compareMinor", () => {
  it("addMinor sums variadic bigints", () => {
    expect(addMinor()).toBe(0n);
    expect(addMinor(100n)).toBe(100n);
    expect(addMinor(100n, 200n, 300n)).toBe(600n);
  });

  it("subMinor returns precise difference", () => {
    expect(subMinor(500n, 100n)).toBe(400n);
    expect(subMinor(0n, 100n)).toBe(-100n);
  });

  it("compareMinor returns -1/0/1", () => {
    expect(compareMinor(0n, 0n)).toBe(0);
    expect(compareMinor(100n, 200n)).toBe(-1);
    expect(compareMinor(200n, 100n)).toBe(1);
  });
});
