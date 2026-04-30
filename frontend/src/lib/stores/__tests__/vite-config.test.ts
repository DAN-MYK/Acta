import { describe, expect, it } from "vitest";
import config from "../../../../../vite.config";

describe("vite dev server config", () => {
  it("ignores generated and temporary directories that trigger noisy full reloads", () => {
    const ignored = config.server?.watch?.ignored;

    expect(ignored).toEqual(
      expect.arrayContaining([
        "**/.tmp*/**",
        "**/target/**",
        "**/src-tauri/target/**",
        "**/dist/**",
        "**/storage/**"
      ])
    );
  });
});
