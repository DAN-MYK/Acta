/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ComponentProps } from "svelte";
import CounterpartyModal from "../CounterpartyModal.svelte";
import type { CounterpartyDraftFormDto } from "../../types";

const mockForm: CounterpartyDraftFormDto = {
  id: "cp-1",
  title: "ТОВ Ромашка",
  name: "ТОВ Ромашка",
  edrpou: "12345678",
  ipn: "",
  iban: "UA123",
  address: "",
  phone: "",
  email: "",
  notes: "",
};

function mount(props: Partial<ComponentProps<CounterpartyModal>>) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new CounterpartyModal({ target, props: props as ComponentProps<CounterpartyModal> });
  return { component, target };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("CounterpartyModal", () => {
  it("renders 'Новий контрагент' title in create mode", () => {
    const { target, component } = mount({ isOpen: true, mode: "create", form: { ...mockForm, id: "" } });
    expect(target.querySelector("#modal-title")?.textContent).toBe("Новий контрагент");
    component.$destroy();
  });

  it("renders 'Редагування контрагента' title in edit mode", () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm });
    expect(target.querySelector("#modal-title")?.textContent).toBe("Редагування контрагента");
    component.$destroy();
  });

  it("shows dirty-confirm block when showCloseConfirm is true", () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm, showCloseConfirm: true });
    expect(target.querySelector("[data-testid='cp-modal-dirty-confirm']")).toBeTruthy();
    component.$destroy();
  });

  it("hides dirty-confirm block when showCloseConfirm is false", () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm, showCloseConfirm: false });
    expect(target.querySelector("[data-testid='cp-modal-dirty-confirm']")).toBeNull();
    component.$destroy();
  });

  it("dispatches closeConfirmed when 'Так, закрити' is clicked", async () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm, showCloseConfirm: true });
    const handler = vi.fn();
    component.$on("closeConfirmed", handler);
    const btn = Array.from(target.querySelectorAll("button")).find(
      (b) => b.textContent?.trim() === "Так, закрити"
    );
    btn?.click();
    await Promise.resolve();
    expect(handler).toHaveBeenCalledOnce();
    component.$destroy();
  });

  it("dispatches closeCancelled when 'Залишитись' is clicked", async () => {
    const { target, component } = mount({ isOpen: true, mode: "edit", form: mockForm, showCloseConfirm: true });
    const handler = vi.fn();
    component.$on("closeCancelled", handler);
    const btn = Array.from(target.querySelectorAll("button")).find(
      (b) => b.textContent?.trim() === "Залишитись"
    );
    btn?.click();
    await Promise.resolve();
    expect(handler).toHaveBeenCalledOnce();
    component.$destroy();
  });

  it("dispatches save when 'Зберегти' is clicked", async () => {
    const { target, component } = mount({ isOpen: true, mode: "create", form: { ...mockForm, id: "" } });
    const handler = vi.fn();
    component.$on("save", handler);
    const btn = Array.from(target.querySelectorAll("button")).find(
      (b) => b.textContent?.trim() === "Зберегти"
    );
    btn?.click();
    await Promise.resolve();
    expect(handler).toHaveBeenCalledOnce();
    component.$destroy();
  });

  it("does not render when isOpen is false", () => {
    const { target, component } = mount({ isOpen: false, mode: "create", form: mockForm });
    expect(target.querySelector(".modal-backdrop")).toBeNull();
    component.$destroy();
  });
});
