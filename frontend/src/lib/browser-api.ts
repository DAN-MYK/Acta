import { browserFixtureInvoke } from "./browser-fixtures";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __ACTA_FORCE_BROWSER_FIXTURES__?: boolean;
  }
}

export function isBrowserFallbackEnabled() {
  if (typeof window === "undefined") {
    return false;
  }

  if (window.__ACTA_FORCE_BROWSER_FIXTURES__) {
    return true;
  }

  return !window.__TAURI_INTERNALS__;
}

export function invokeInBrowser<T>(command: string, payload?: Record<string, unknown>) {
  return browserFixtureInvoke<T>(command, payload);
}
