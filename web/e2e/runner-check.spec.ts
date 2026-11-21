import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { test, expect } from "@playwright/test";

// Runs only under scripts/e2e-runner-check.sh, which sets
// SIDEREAL_E2E_RUNNER_CHECK (web/playwright.config.ts ignores this file
// otherwise). The test opens the web shell, says it is holding by writing
// `holding` in the run folder, and keeps the browser open until the check
// writes `release`, so the check can signal the runner while Vite and
// Chromium run. With SIDEREAL_E2E_RUNNER_CHECK=fail it fails instead.
const runDir = process.env.SIDEREAL_E2E_RUN_DIR ?? "";

test("holds the browser open for the runner check", async ({ page }) => {
  test.setTimeout(600_000);
  await page.goto("/");
  await expect(page.getByRole("strong")).toHaveText(/healthy/i);
  expect(
    process.env.SIDEREAL_E2E_RUNNER_CHECK,
    "the runner check asked this test to fail",
  ).not.toBe("fail");

  writeFileSync(join(runDir, "holding"), "");
  await expect
    .poll(() => existsSync(join(runDir, "release")), {
      timeout: 590_000,
      intervals: [200],
    })
    .toBe(true);

  // After a restart, the page reaches the new server through Vite.
  await page.reload();
  await expect(page.getByRole("strong")).toHaveText(/healthy/i);
});
