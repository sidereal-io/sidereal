import { test, expect } from '@playwright/test';

test.describe('Error Handling', () => {
  test('should show 404 page for unknown routes', async ({ page }) => {
    await page.goto('/non-existent-page');
    await expect(page.getByText('404 Page Not Found')).toBeVisible();
  });

  test('should display 404 card with error icon', async ({ page }) => {
    await page.goto('/some-random-route');
    // The not-found page has an AlertCircle icon and a Card
    await expect(page.getByText('404 Page Not Found')).toBeVisible();
    await expect(page.getByText(/forget to add the page/i)).toBeVisible();
  });

  test('should render header even when API is blocked', async ({ page }) => {
    await page.route('**/api/**', (route) => route.abort());
    await page.goto('/');
    await expect(page.locator('header')).toBeVisible();
    await expect(page.locator('header h1')).toHaveText('Sidereal');
  });

  test('should render search filters when API is blocked', async ({ page }) => {
    await page.route('**/api/**', (route) => route.abort());
    await page.goto('/');
    await expect(page.getByPlaceholder('Search astrophotography images...')).toBeVisible();
  });

  test('should render equipment page when API is blocked', async ({ page }) => {
    await page.route('**/api/**', (route) => route.abort());
    await page.goto('/equipment');
    await expect(page.getByRole('heading', { name: /equipment catalog/i })).toBeVisible();
  });

  test('should render admin page when API is blocked', async ({ page }) => {
    await page.route('**/api/**', (route) => route.abort());
    await page.goto('/admin');
    await expect(page.getByRole('heading', { name: /admin settings/i })).toBeVisible();
  });

  test('should render plate solving page when API is blocked', async ({ page }) => {
    await page.route('**/api/**', (route) => route.abort());
    await page.goto('/plate-solving');
    await expect(page.getByRole('heading', { name: /plate solving/i })).toBeVisible();
  });
});
