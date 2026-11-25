import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test, expect, type Page } from "@playwright/test";

// The first-image journey, on the stack that `mise run e2e` makes for this
// run: Nova claims the instance, signs in, uploads a real PNG and JPEG in
// one pick, uploads the PNG's bytes again under another name and is told it is already
// there, and still sees exactly the two images after the runner restarts the
// server. The Rust process test (server/crates/server/tests/journey.rs)
// checks ids and bytes; this test checks what Nova sees.
const runDir = process.env.SIDEREAL_E2E_RUN_DIR ?? "";
const runnerPid = Number(process.env.SIDEREAL_E2E_RUNNER_PID);
const fixtures = fileURLToPath(
  new URL("../../server/crates/server/tests/fixtures/", import.meta.url),
);
const images = ["nebula.png", "nebula.jpg"];

// Serial with no retries: a second attempt would find the instance claimed.
test.describe.configure({ mode: "serial", retries: 0 });

async function signIn(page: Page, button: "Claim" | "Sign in") {
  await page.getByLabel("Username").fill("nova");
  await page.getByLabel("Password").fill("correct-horse-battery");
  await page.getByRole("button", { name: button }).click();
}

// The image is on screen and the browser decoded it.
async function expectShown(page: Page, name: string) {
  await expect(page.getByText(name, { exact: true })).toBeVisible();
  const image = page.getByRole("img", { name });
  await expect(image).toBeVisible();
  await expect
    .poll(() =>
      image.evaluate(
        (element: HTMLImageElement) =>
          element.complete && element.naturalWidth > 0,
      ),
    )
    .toBe(true);
}

test("Nova uploads a PNG and a JPEG and sees them after a restart", async ({
  page,
}) => {
  expect(runDir, "run this suite with `mise run e2e`").not.toBe("");
  expect(Number.isInteger(runnerPid) && runnerPid > 0).toBe(true);
  // The restart may wait up to 300 seconds for the new server.
  test.setTimeout(420_000);

  await page.goto("/");
  await signIn(page, "Claim");
  await signIn(page, "Sign in");
  await expect(
    page.getByText("No images yet. Upload a PNG or JPEG to start."),
  ).toBeVisible();

  // Both images in one pick, through the browser's multiple-file picker.
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Upload" }).click();
  await (await chooser).setFiles(images.map((name) => join(fixtures, name)));
  for (const name of images) {
    await expect(page.getByText(`${name} was uploaded.`)).toBeVisible();
    await expectShown(page, name);
  }

  // The same bytes as nebula.png, under another name, find that image.
  const again = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Upload" }).click();
  const picker = await again;
  await picker.setFiles({
    name: "m31-final.png",
    mimeType: "image/png",
    buffer: readFileSync(join(fixtures, "nebula.png")),
  });
  await expect(
    page.getByText("m31-final.png is already in the library as nebula.png."),
  ).toBeVisible();
  await expect(page.getByRole("img")).toHaveCount(images.length);

  // Ask the runner to restart the server, and wait until the new one is up.
  process.kill(runnerPid, "SIGUSR1");
  await expect
    .poll(() => existsSync(join(runDir, "ready-1")), {
      timeout: 330_000,
      intervals: [200],
    })
    .toBe(true);

  await page.reload();
  for (const name of images) await expectShown(page, name);
  await expect(page.getByRole("img")).toHaveCount(images.length);
});
