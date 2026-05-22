/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ComponentProps } from "svelte";
import DocumentFilters from "../documents/DocumentFilters.svelte";

function mount(props: Partial<ComponentProps<DocumentFilters>> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new DocumentFilters({
    target,
    props: {
      open: true,
      loading: false,
      dateFrom: null,
      dateTo: null,
      statusFilter: [],
      amountMin: null,
      amountMax: null,
      overdueOnly: false,
      counterpartyFilterId: null,
      counterparties: [
        { id: "counterparty-1", name: "ТОВ Ромашка" },
        { id: "counterparty-2", name: "ФОП Тест" }
      ],
      ...props
    } as ComponentProps<DocumentFilters>
  });

  return { component, target };
}

afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = "";
});

describe("DocumentFilters", () => {
  it("dispatches apply with the current draft values", async () => {
    const { component, target } = mount();
    const onApply = vi.fn();
    component.$on("apply", onApply);

    const panel = target.querySelector('[data-testid="documents-filter-panel"]') as HTMLElement;
    expect(panel).toBeTruthy();

    const dateInputs = panel.querySelectorAll('input[type="date"]');
    (dateInputs[0] as HTMLInputElement).value = "2026-05-01";
    dateInputs[0].dispatchEvent(new Event("input", { bubbles: true }));
    (dateInputs[1] as HTMLInputElement).value = "2026-05-21";
    dateInputs[1].dispatchEvent(new Event("input", { bubbles: true }));

    const amountInputs = panel.querySelectorAll('input[inputmode="decimal"]');
    (amountInputs[0] as HTMLInputElement).value = "1000,50";
    amountInputs[0].dispatchEvent(new Event("input", { bubbles: true }));
    (amountInputs[1] as HTMLInputElement).value = "2500,00";
    amountInputs[1].dispatchEvent(new Event("input", { bubbles: true }));

    const counterpartySelect = panel.querySelector('[data-testid="documents-counterparty-filter"]') as HTMLSelectElement;
    counterpartySelect.value = "counterparty-2";
    counterpartySelect.dispatchEvent(new Event("change", { bubbles: true }));

    const status = panel.querySelector('input[type="checkbox"]') as HTMLInputElement;
    status.checked = true;
    status.dispatchEvent(new Event("change", { bubbles: true }));

    (panel.querySelector(".btn-primary") as HTMLButtonElement).click();
    await Promise.resolve();

    expect(onApply).toHaveBeenCalledWith(
      expect.objectContaining({
        detail: {
          dateFrom: "2026-05-01",
          dateTo: "2026-05-21",
          statusFilter: ["draft"],
          amountMin: "1000.50",
          amountMax: "2500.00",
          counterpartyFilterId: "counterparty-2"
        }
      })
    );

    component.$destroy();
  });

  it("uses the local calendar date for quick period presets", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-20T21:30:00.000Z"));

    const { component, target } = mount();
    const panel = target.querySelector('[data-testid="documents-filter-panel"]') as HTMLElement;

    (panel.querySelector(".filter-panel-subpresets button") as HTMLButtonElement).click();
    await Promise.resolve();

    const dateInputs = panel.querySelectorAll('input[type="date"]');
    expect((dateInputs[0] as HTMLInputElement).value).toBe("2026-05-21");
    expect((dateInputs[1] as HTMLInputElement).value).toBe("2026-05-21");

    component.$destroy();
  });

  it("rejects non-decimal amount syntax instead of coercing it through Number", async () => {
    const { component, target } = mount();
    const onApply = vi.fn();
    component.$on("apply", onApply);

    const panel = target.querySelector('[data-testid="documents-filter-panel"]') as HTMLElement;
    const amountInputs = panel.querySelectorAll('input[inputmode="decimal"]');
    const applyButton = panel.querySelector('[data-testid="documents-filter-apply"]') as HTMLButtonElement;

    for (const invalidValue of ["1e6", "0x10", "Infinity"]) {
      (amountInputs[0] as HTMLInputElement).value = invalidValue;
      amountInputs[0].dispatchEvent(new Event("input", { bubbles: true }));
      await Promise.resolve();

      expect(applyButton.disabled).toBe(true);
      applyButton.click();
      expect(onApply).not.toHaveBeenCalled();
    }

    component.$destroy();
  });
});
