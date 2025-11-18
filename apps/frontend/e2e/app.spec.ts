import { test, expect } from '@playwright/test';

test('should display hello world', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByText('Sidereal Frontend')).toBeVisible();
  await expect(page.getByText('Hello World!')).toBeVisible();
});
