/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import SkeletonRow from "../SkeletonRow.svelte";

function renderRow(props: Record<string, unknown> = {}) {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new SkeletonRow({ target, props });
  return { component, target };
}

describe("SkeletonRow", () => {
  it("renders the requested row count", () => {
    const { component, target } = renderRow({ count: 3 });

    expect(target.querySelectorAll('[data-testid="skeleton-row-item"]')).toHaveLength(3);

    component.$destroy();
  });

  it("renders icon block in default variant", () => {
    const { component, target } = renderRow();

    expect(target.querySelector('[data-testid="skeleton-row-icon"]')).toBeTruthy();

    component.$destroy();
  });

  it("omits icon block in compact variant", () => {
    const { component, target } = renderRow({ variant: "compact" });

    expect(target.querySelector('[data-testid="skeleton-row-icon"]')).toBeNull();
    const row = target.querySelector('[data-testid="skeleton-row-item"]');
    const copy = target.querySelector(".skeleton-copy");
    const meta = target.querySelector(".skeleton-meta");

    expect(row?.classList.contains("skeleton-row-compact")).toBe(true);
    expect(copy?.getAttribute("style")).toContain("grid-column:1");
    expect(meta?.getAttribute("style")).toContain("grid-column:2");

    component.$destroy();
  });
});
