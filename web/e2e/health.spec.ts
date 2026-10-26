import { test, expect } from "@playwright/test";

test("shows the backend as healthy on first load", async ({ page }) => {
  await page.goto("/");

  // HealthStatus renders "Server: healthy" once `/healthz` returns 200 with
  // `{"status":"ok"}`. The `<strong>` makes this a single token Playwright
  // can match without coupling to layout.
  await expect(page.getByRole("strong")).toHaveText(/healthy/i);
});
