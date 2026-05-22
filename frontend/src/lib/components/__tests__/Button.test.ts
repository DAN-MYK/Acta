/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it } from "vitest";
import Button from "../Button.svelte";

afterEach(() => {
  document.body.innerHTML = "";
});

describe("Button", () => {
  it("keeps a disabled busy state with an accessible loading label", () => {
    const target = document.createElement("div");
    document.body.appendChild(target);
    const component = new Button({
      target,
      props: {
        loading: true,
        loadingLabel: "Збереження..."
      }
    });

    const button = target.querySelector("button") as HTMLButtonElement;

    expect(button.disabled).toBe(true);
    expect(button.getAttribute("aria-busy")).toBe("true");
    expect(button.textContent).toContain("Збереження...");
    expect(button.querySelector('[data-testid="button-spinner"]')).toBeTruthy();

    component.$destroy();
  });
});
