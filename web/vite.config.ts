import react from "@vitejs/plugin-react";
import { configDefaults, defineConfig } from "vitest/config";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // Keep earlier output, such as a cargo error from `just dev`, on screen.
  clearScreen: false,
  server: {
    // Stop with an error when 5173 is taken, instead of serving on another
    // port where a contributor would not look.
    strictPort: true,
    // The web shell needs no cross-origin access, so other local apps
    // cannot read the server through the proxy.
    cors: false,
    // The browser asks the web shell's own origin; the dev server forwards
    // to the server.
    proxy: {
      "/healthz": "http://localhost:5000",
    },
  },
  // Unit tests run once in a simulated browser DOM, with no server.
  test: {
    environment: "jsdom",
    // Vitest replaces `exclude` rather than extending it, so the defaults
    // (dist/, .git/ and the like) have to be spread back in explicitly or a
    // stale build would be collected as tests. The e2e folders are not
    // Vitest's to run either; `pnpm e2e` owns them.
    exclude: [
      ...configDefaults.exclude,
      "e2e/**",
      "test-results/**",
      "playwright-report/**",
    ],
  },
});
