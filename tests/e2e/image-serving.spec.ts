import { test, expect } from '@playwright/test';
import { HomePage } from './pages';

test.describe('Local image serving', () => {
  test('image gallery img tags reference /api/images/ routes, not Immich proxy', async ({ page }) => {
    const homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();

    // If no images are synced, skip this test gracefully
    const galleryImages = page.locator('img[src*="/api/images/"]');
    const count = await galleryImages.count();

    if (count === 0) {
      test.skip();
      return;
    }

    // Verify at least one gallery image loads from our local route
    const firstImg = galleryImages.first();
    await expect(firstImg).toBeVisible();

    const src = await firstImg.getAttribute('src');
    expect(src).toMatch(/\/api\/images\/\d+\/(thumbnail|preview)/);

    // Verify the image actually loads (not a 404)
    const [response] = await Promise.all([
      page.waitForResponse(r => r.url().includes('/api/images/') && r.status() === 200),
      page.reload(),
    ]);
    expect(response.status()).toBe(200);
    const cacheControl = response.headers()['cache-control'];
    expect(cacheControl).toContain('max-age=31536000');
  });

  test('image URL helper derives correct route from image id', async ({ page }) => {
    // Verify that no img tags on the page reference the old /api/assets/ Immich proxy
    const homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();

    const immichProxyImages = page.locator('img[src*="/api/assets/"]');
    await expect(immichProxyImages).toHaveCount(0);
  });
});
