/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { tick } from "svelte";
import PaymentGroups from "../payments/PaymentGroups.svelte";
import type { PaymentItemDto } from "../../types";

const unmatchedPayment: PaymentItemDto = {
  id: "payment-unmatched",
  date: "2026-05-20",
  counterpartyId: "counterparty-1",
  counterparty: "ТОВ Приклад",
  amountStr: "1 200,00",
  direction: "in",
  matchedDoc: "",
  account: "UA123"
};

const matchedPayment: PaymentItemDto = {
  id: "payment-matched",
  date: "2026-05-21",
  counterpartyId: "counterparty-2",
  counterparty: "ФОП Тест",
  amountStr: "-800,00",
  direction: "out",
  matchedDoc: "Акт ACT-001",
  account: "UA456"
};

interface PaymentGroupsProps {
  unmatchedPayments: PaymentItemDto[];
  matchedPayments: PaymentItemDto[];
  initialLoading: boolean;
  loading: boolean;
  activePaymentId: string | null;
  onOpenEditor: (payment: PaymentItemDto) => void;
  onReconcile: (paymentId: string) => void;
  onUnreconcile: (paymentId: string) => void;
}

function mount(props: Partial<PaymentGroupsProps> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new PaymentGroups({
    target,
    props: {
      unmatchedPayments: [unmatchedPayment],
      matchedPayments: [matchedPayment],
      initialLoading: false,
      loading: false,
      activePaymentId: null,
      onOpenEditor: vi.fn(),
      onReconcile: vi.fn(),
      onUnreconcile: vi.fn(),
      ...props
    }
  });

  return { component, target };
}

describe("PaymentGroups", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders unmatched and matched groups with counts and payment actions", async () => {
    const onOpenEditor = vi.fn();
    const onReconcile = vi.fn();
    const onUnreconcile = vi.fn();
    const { component, target } = mount({ onOpenEditor, onReconcile, onUnreconcile });
    await tick();

    expect(target.querySelector('[data-testid="payments-unmatched-group"]')?.textContent).toContain("Потребують звірки");
    expect(target.querySelector('[data-testid="payments-unmatched-group"]')?.textContent).toContain("1");
    expect(target.querySelector('[data-testid="payments-matched-group"]')?.textContent).toContain("Вже зведені");
    expect(target.querySelector('[data-testid="payments-matched-group"]')?.textContent).toContain("1");
    expect(target.textContent).toContain("ТОВ Приклад");
    expect(target.textContent).toContain("ФОП Тест");

    const openButtons = Array.from(target.querySelectorAll(".payment-row-main")) as HTMLButtonElement[];
    openButtons[0].click();
    expect(onOpenEditor).toHaveBeenCalledWith(unmatchedPayment);

    const reconcileButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Звести")
    ) as HTMLButtonElement | undefined;
    reconcileButton?.click();
    expect(onReconcile).toHaveBeenCalledWith("payment-unmatched");

    const unreconcileButton = Array.from(target.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("Зняти зведення")
    ) as HTMLButtonElement | undefined;
    unreconcileButton?.click();
    expect(onUnreconcile).toHaveBeenCalledWith("payment-matched");

    component.$destroy();
  });

  it("shows loading skeletons instead of payment rows during initial loading", async () => {
    const { component, target } = mount({ initialLoading: true });
    await tick();

    expect(target.querySelectorAll('[data-testid="skeleton-row-item"]').length).toBeGreaterThan(0);
    expect(target.querySelector(".payment-row")).toBeNull();

    component.$destroy();
  });
});
