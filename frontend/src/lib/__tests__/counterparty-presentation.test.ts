import { describe, expect, it } from "vitest";
import {
  getCounterpartyFinancialSummary,
  getCounterpartyLastContactLabel,
  getCounterpartyOverdueDocumentsLabel,
  getCounterpartyRiskLabel,
  getCounterpartyScenarioDescription,
  getCounterpartyScenarioTitle
} from "../counterpartyPresentation";

describe("counterpartyPresentation", () => {
  it("formats reused overdue and contact labels through canonical ui helpers", () => {
    expect(getCounterpartyOverdueDocumentsLabel(1)).toBe("1 \u0434\u043e\u043a\u0443\u043c\u0435\u043d\u0442");
    expect(getCounterpartyOverdueDocumentsLabel(3)).toBe("3 \u0434\u043e\u043a\u0443\u043c\u0435\u043d\u0442\u0438");
    expect(getCounterpartyLastContactLabel(0)).toBe("\u0441\u044c\u043e\u0433\u043e\u0434\u043d\u0456");
    expect(getCounterpartyLastContactLabel(5)).toBe("5 \u0434\u043d\u0456\u0432 \u0442\u043e\u043c\u0443");
  });

  it("returns stable risk labels for healthy and overdue counterparties", () => {
    expect(getCounterpartyRiskLabel(0)).toBe("\u041f\u0440\u0430\u0446\u044e\u0454 \u0441\u0442\u0430\u0431\u0456\u043b\u044c\u043d\u043e");
    expect(getCounterpartyRiskLabel(2)).toBe(
      "\u041f\u043e\u0442\u0440\u0435\u0431\u0443\u0454 \u0443\u0432\u0430\u0433\u0438: \u043f\u0440\u043e\u0441\u0442\u0440\u043e\u0447\u0435\u043d\u043e 2 \u0434\u043e\u043a\u0443\u043c\u0435\u043d\u0442\u0438"
    );
  });

  it("chooses the next action title by overdue, stale contact, and empty-document priority", () => {
    expect(getCounterpartyScenarioTitle(1, 4, 2)).toBe(
      "\u0417\u0430\u043a\u0440\u0438\u0442\u0438 \u043f\u0440\u043e\u0441\u0442\u0440\u043e\u0447\u043a\u0443"
    );
    expect(getCounterpartyScenarioTitle(0, 21, 2)).toBe(
      "\u041e\u043d\u043e\u0432\u0438\u0442\u0438 \u043a\u043e\u043d\u0442\u0430\u043a\u0442"
    );
    expect(getCounterpartyScenarioTitle(0, 5, 0)).toBe(
      "\u0417\u0430\u043f\u0443\u0441\u0442\u0438\u0442\u0438 \u043f\u0435\u0440\u0448\u0438\u0439 \u0434\u043e\u043a\u0443\u043c\u0435\u043d\u0442"
    );
    expect(getCounterpartyScenarioTitle(0, 5, 3)).toBe(
      "\u0422\u0440\u0438\u043c\u0430\u0442\u0438 \u0441\u0446\u0435\u043d\u0430\u0440\u0456\u0439 \u0443 \u0440\u0443\u0441\u0456"
    );
  });

  it("describes each counterparty scenario branch with the expected business copy", () => {
    expect(getCounterpartyScenarioDescription(2, "19 000,00 \u0433\u0440\u043d", 3, 4)).toContain(
      "\u0404 2 \u0434\u043e\u043a\u0443\u043c\u0435\u043d\u0442\u0438 \u043d\u0430 19 000,00 \u0433\u0440\u043d"
    );
    expect(getCounterpartyScenarioDescription(0, "0,00 \u0433\u0440\u043d", 21, 4)).toContain(
      "\u041e\u0441\u0442\u0430\u043d\u043d\u0456\u0439 \u043a\u043e\u043d\u0442\u0430\u043a\u0442 \u0431\u0443\u0432 21 \u0434\u043d\u0456\u0432 \u0442\u043e\u043c\u0443"
    );
    expect(getCounterpartyScenarioDescription(0, "0,00 \u0433\u0440\u043d", 4, 0)).toContain(
      "\u0449\u0435 \u043d\u0435\u043c\u0430\u0454 \u0430\u043a\u0442\u0438\u0432\u043d\u0438\u0445 \u0434\u043e\u043a\u0443\u043c\u0435\u043d\u0442\u0456\u0432"
    );
    expect(getCounterpartyScenarioDescription(0, "0,00 \u0433\u0440\u043d", 4, 2)).toContain(
      "\u041a\u043e\u043d\u0442\u0440\u0430\u0433\u0435\u043d\u0442 \u0431\u0435\u0437 \u043f\u0440\u043e\u0441\u0442\u0440\u043e\u0447\u043e\u043a"
    );
  });

  it("summarizes financial state by overdue risk first, then balance sign", () => {
    expect(getCounterpartyFinancialSummary(false, 1)).toBe(
      "\u041f\u043e\u0442\u043e\u0447\u043d\u0438\u0439 \u0441\u0442\u0430\u043d \u043f\u043e\u0442\u0440\u0435\u0431\u0443\u0454 \u0440\u0443\u0447\u043d\u043e\u0457 \u0443\u0432\u0430\u0433\u0438."
    );
    expect(getCounterpartyFinancialSummary(true, 0)).toBe(
      "\u0411\u0430\u043b\u0430\u043d\u0441 \u0432\u0456\u0434'\u0454\u043c\u043d\u0438\u0439, \u0430\u043b\u0435 \u0431\u0435\u0437 \u043f\u0440\u043e\u0441\u0442\u0440\u043e\u0447\u043e\u043a."
    );
    expect(getCounterpartyFinancialSummary(false, 0)).toBe(
      "\u0411\u0430\u043b\u0430\u043d\u0441 \u043f\u0456\u0434 \u043a\u043e\u043d\u0442\u0440\u043e\u043b\u0435\u043c, \u043a\u0440\u0438\u0442\u0438\u0447\u043d\u0438\u0445 \u0441\u0438\u0433\u043d\u0430\u043b\u0456\u0432 \u043d\u0435\u043c\u0430\u0454."
    );
  });
});
