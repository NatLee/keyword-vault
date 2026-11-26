/// <reference types="vitest/config" />
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// 開發時把 /api 轉給後端；後端預設在 8686（KV_BIND），要換 port 就設 KV_API_URL。
const apiTarget = process.env.KV_API_URL ?? "http://127.0.0.1:8686";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      // Preserve the browser Host so the API can validate same-origin writes.
      "/api": { target: apiTarget, changeOrigin: false },
    },
  },
  build: {
    outDir: "dist",
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    css: false,
  },
});
