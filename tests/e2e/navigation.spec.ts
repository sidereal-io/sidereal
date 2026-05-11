import { test, expect } from '@playwright/test';
import { HomePage, EquipmentPage, PlateSolvingPage, AdminPage, SkyMapPage } from './pages';

test.describe('Navigation', () => {
  test('should navigate from home to equipment page', async ({ page }) => {
    const homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();

    await homePage.navigateTo('equipment');
    const equipmentPage = new EquipmentPage(page);
    await equipmentPage.verifyPageLoaded();
  });

  test('should navigate from home to plate solving page', async ({ page }) => {
    const homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();

    await homePage.navigateTo('plateSolving');
    const plateSolvingPage = new PlateSolvingPage(page);
    await plateSolvingPage.verifyPageLoaded();
  });

  test('should navigate from home to admin page', async ({ page }) => {
    const homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();

    await homePage.navigateTo('admin');
    const adminPage = new AdminPage(page);
    await adminPage.verifyPageLoaded();
  });

  test('should navigate across all pages sequentially', async ({ page }) => {
    const homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();

    await homePage.navigateTo('equipment');
    await expect(page).toHaveURL(/\/equipment/);

    const equipmentPage = new EquipmentPage(page);
    await equipmentPage.navigateTo('plateSolving');
    await expect(page).toHaveURL(/\/plate-solving/);

    const plateSolvingPage = new PlateSolvingPage(page);
    await plateSolvingPage.navigateTo('admin');
    await expect(page).toHaveURL(/\/admin/);

    const adminPage = new AdminPage(page);
    await adminPage.navigateTo('gallery');
    await expect(page).toHaveURL(/\/$/);
  });

  test('should navigate home by clicking logo', async ({ page }) => {
    const homePage = new HomePage(page);
    await homePage.goto();
    await homePage.navigateTo('equipment');
    await expect(page).toHaveURL(/\/equipment/);

    // Click the logo/brand link to go home
    await page.locator('header a[href="/"]').first().click();
    await page.waitForURL('**/');
    await expect(page).toHaveURL(/\/$/);
  });

  test('should highlight active navigation link', async ({ page }) => {
    const homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();

    // Gallery link should be active (text-foreground class)
    const galleryLink = homePage.navigationLinks.gallery;
    await expect(galleryLink).toHaveClass(/text-foreground/);

    // Equipment link should be inactive
    const equipmentLink = homePage.navigationLinks.equipment;
    await expect(equipmentLink).toHaveClass(/text-muted-foreground/);
  });
});

test.describe('Header - Consistent Across Pages', () => {
  const pages = [
    { name: 'Home', url: '/' },
    { name: 'Equipment', url: '/equipment' },
    { name: 'Plate Solving', url: '/plate-solving' },
    { name: 'Admin', url: '/admin' },
    { name: 'Sky Map', url: '/sky-map' },
  ];

  for (const p of pages) {
    test(`should display header on ${p.name} page`, async ({ page }) => {
      await page.goto(p.url);
      await expect(page.locator('header')).toBeVisible();
      await expect(page.locator('header h1')).toHaveText('Sidereal');
    });

    test(`should display Sync Immich button on ${p.name} page`, async ({ page }) => {
      await page.goto(p.url);
      await expect(page.getByRole('button', { name: /sync immich/i })).toBeVisible();
    });

    test(`should display GitHub link on ${p.name} page`, async ({ page }) => {
      await page.goto(p.url);
      await expect(page.locator('a[href="https://github.com/mstelz/sidereal"]')).toBeVisible();
    });
  }
});
