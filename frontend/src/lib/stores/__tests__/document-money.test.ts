import { describe, expect, it } from "vitest";
import { formatDocumentDraftTotal, formatDocumentItemTotal } from "../../documentMoney";
import type { DocumentDraftItemDto } from "../../types";

describe("documentMoney", () => {
  it("formats item totals with Decimal-style string inputs", () => {
    expect(formatDocumentItemTotal("2", "1500,25")).toBe("3 000,50 грн");
    expect(formatDocumentItemTotal("1.5", "1000")).toBe("1 500,00 грн");
    expect(formatDocumentItemTotal("bad", "1000")).toBe("—");
  });

  it("rounds item totals to two money decimals", () => {
    expect(formatDocumentItemTotal("3", "0,333")).toBe("1,00 грн");
  });

  it("sums draft item totals without floating point math", () => {
    const items: DocumentDraftItemDto[] = [
      { description: "Послуга", unit: "год", quantity: "2", price: "1500,25" },
      { description: "Матеріал", unit: "шт", quantity: "3", price: "100,10" },
      { description: "Порожній рядок", unit: "", quantity: "", price: "" }
    ];

    expect(formatDocumentDraftTotal(items)).toBe("3 300,80 грн");
  });
});
