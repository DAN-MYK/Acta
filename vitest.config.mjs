import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["frontend/src/lib/stores/__tests__/**/*.test.ts"]
  }
});
