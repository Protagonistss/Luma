import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "web"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./web/test/setup.ts"],
    globals: true,
  },
});
