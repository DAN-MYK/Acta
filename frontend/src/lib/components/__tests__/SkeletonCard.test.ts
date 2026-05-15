/**
 * @vitest-environment jsdom
 */
import { expect, it } from "vitest";
import SkeletonCard from "../SkeletonCard.svelte";

it("renders the requested card count", () => {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new SkeletonCard({ target, props: { count: 4 } });

  expect(target.querySelector('[data-testid="skeleton-card-grid"]')).toBeTruthy();
  expect(target.querySelectorAll('[data-testid="skeleton-card-item"]')).toHaveLength(4);

  component.$destroy();
});

it("marks decorative placeholders as hidden from assistive tech", () => {
  const target = document.createElement("div");
  document.body.appendChild(target);
  const component = new SkeletonCard({ target, props: { count: 1 } });

  const card = target.querySelector('[data-testid="skeleton-card-item"]');

  expect(card?.getAttribute("aria-hidden")).toBe("true");
  expect(card?.tagName).toBe("DIV");

  component.$destroy();
});
