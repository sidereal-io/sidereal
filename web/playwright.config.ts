import { defineConfig, devices } from "@playwright/test";

// End-to-end tests run against the real stack: the Rust server on 5000 and
// Vite on 5173. Vite proxies /healthz to the server, so the browser only ever
// talks to Vite while the server sits behind the proxy exactly as it does in
// production.
const SERVER_URL = "http://localhost:5000";
const WEB_URL = "http://localhost:5173";

// A developer almost always has `just dev` running already. Reusing the
// existing process keeps the suite fast; when nothing is listening the
// command below starts it and Playwright waits for the port. An explicit
// PLAYWRIGHT_REUSE_SERVER=0 forces a fresh stack and =1 forces reuse; unset
// means "reuse locally, never on CI".
const reuseExisting = process.env.PLAYWRIGHT_REUSE_SERVER
  ? process.env.PLAYWRIGHT_REUSE_SERVER === "1"
  : !process.env.CI;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  // Single worker on CI: the stack is a single shared server, and parallel
  // browsers against it only add flake, not coverage.
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI
    ? [["github"], ["html", { open: "never" }]]
    : [["list"]],
  use: {
    baseURL: WEB_URL,
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: [
    {
      // Same command `just server` uses, so a local run and a CI run boot
      // the server the same way.
      command: "cargo run -p sidereal-server --manifest-path server/Cargo.toml",
      url: `${SERVER_URL}/healthz`,
      reuseExistingServer: reuseExisting,
      timeout: 300_000,
      cwd: "..",
      stdout: "pipe",
      stderr: "pipe",
    },
    {
      command: "pnpm dev --port 5173 --strictPort",
      url: WEB_URL,
      reuseExistingServer: reuseExisting,
      timeout: 120_000,
      stdout: "pipe",
      stderr: "pipe",
    },
  ],
});
