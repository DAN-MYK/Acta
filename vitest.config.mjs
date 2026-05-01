import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  test: {
    environment: "node",
    include: [
      "e2e-tests/__tests__/**/*.test.ts",
      "frontend/src/lib/stores/__tests__/**/*.test.ts",
      "frontend/src/lib/screens/__tests__/**/*.test.ts"
    ]
  }
});
