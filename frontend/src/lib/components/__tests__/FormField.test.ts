/**
 * @vitest-environment jsdom
 */
// @ts-ignore Node typings are not included in the frontend test tsconfig.
import { readFileSync } from "fs";
import { afterEach, describe, expect, it } from "vitest";
import FormField from "../FormField.svelte";

afterEach(() => {
  document.body.innerHTML = "";
});

describe("FormField", () => {
  it("exposes stable description ids and slot props for accessible controls", () => {
    const target = document.createElement("div");
    document.body.appendChild(target);
    const component = new FormField({
      target,
      props: {
        id: "amount",
        label: "Сума",
        error: "Сума обов'язкова"
      }
    });

    const message = target.querySelector(".error-text");
    const source = readFileSync("frontend/src/lib/components/FormField.svelte", "utf8");

    expect(message?.id).toBe("amount-error");
    expect(message?.getAttribute("role")).toBe("alert");
    expect(source).toContain("<slot describedBy={ariaDescribedBy} invalid={hasError} />");

    component.$destroy();
  });
});
