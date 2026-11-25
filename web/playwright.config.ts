import { defineConfig, devices } from "@playwright/test";

// End-to-end tests run against a stack that `mise run e2e` (scripts/e2e.sh)
// makes for one run: its own database, storage folder and server, on ports
// derived from the checkout. The runner starts the server and passes its
// port here; this file starts only Vite, which forwards /healthz and /api
// to that server. The browser talks only to Vite, as it does in
// development.
const serverPort = process.env.SIDEREAL_E2E_SERVER_PORT;
const webPort = process.env.SIDEREAL_E2E_WEB_PORT;
if (!serverPort || !webPort) {
  throw new Error(
    "Run the end-to-end suite with `mise run e2e`: it starts the suite's own server and sets SIDEREAL_E2E_SERVER_PORT and SIDEREAL_E2E_WEB_PORT.",
  );
}
const WEB_URL = `http://localhost:${webPort}`;

// scripts/e2e-runner-check.sh sets SIDEREAL_E2E_RUNNER_CHECK to run only
// e2e/runner-check.spec.ts, which holds the browser open while the check
// signals the runner. Other runs never see that file.
const runnerCheck = "runner-check.spec.ts";
const checkingRunner = Boolean(process.env.SIDEREAL_E2E_RUNNER_CHECK);

export default defineConfig({
  testDir: "./e2e",
  ...(checkingRunner
    ? { testMatch: runnerCheck }
    : { testIgnore: `**/${runnerCheck}` }),
  // One worker and no parallel tests, everywhere: the stack is one server,
  // and the upload journey restarts it while it runs.
  fullyParallel: false,
  workers: 1,
  forbidOnly: Boolean(process.env.CI),
  // No retries: a retried upload journey would find the instance already
  // claimed. A failure fails the run once, with its first attempt's trace.
  retries: 0,
  reporter: process.env.CI
    ? [["github"], ["html", { open: "never" }]]
    : [["list"]],
  use: {
    baseURL: WEB_URL,
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    // A taken port stops the run instead of reusing whatever answers there.
    command: `pnpm dev --port ${webPort} --strictPort`,
    url: WEB_URL,
    reuseExistingServer: false,
    timeout: 120_000,
    env: { SIDEREAL_DEV_PROXY_TARGET: `http://127.0.0.1:${serverPort}` },
    stdout: "pipe",
    stderr: "pipe",
  },
});
