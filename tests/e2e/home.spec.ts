import { test, expect } from '@playwright/test';
import { HomePage } from './pages';

test.describe('Home Page', () => {
  let homePage: HomePage;

  test.beforeEach(async ({ page }) => {
    homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();
  });

  test('should load with correct page title', async () => {
    await expect(homePage.page).toHaveTitle('Sidereal');
  });

  test('should display header with app title', async () => {
    await expect(homePage.header).toBeVisible();
    await expect(homePage.appTitle).toHaveText('Sidereal');
  });

  test('should display logo image in header', async () => {
    const logo = homePage.page.locator('header img[alt="Sidereal Logo"]');
    await expect(logo).toBeVisible();
  });

  test('should display navigation links', async () => {
    await homePage.verifyNavigation();
  });

  test('should display Sync Immich button', async () => {
    await expect(homePage.syncButton).toBeVisible();
  });

  test('should display GitHub link', async () => {
    await expect(homePage.githubLink).toBeVisible();
    await expect(homePage.githubLink).toHaveAttribute('target', '_blank');
  });

  test('should display admin settings icon link', async () => {
    await expect(homePage.navigationLinks.admin).toBeVisible();
  });

  test('should display search input', async () => {
    await expect(homePage.searchInput).toBeVisible();
    await expect(homePage.searchInput).toHaveAttribute('placeholder', 'Search astrophotography images...');
  });

  test('should accept and retain search input', async () => {
    await homePage.search('Orion Nebula');
    await expect(homePage.searchInput).toHaveValue('Orion Nebula');
  });

  test('should clear search input', async () => {
    await homePage.search('test');
    await expect(homePage.searchInput).toHaveValue('test');
    await homePage.searchInput.clear();
    await expect(homePage.searchInput).toHaveValue('');
  });

  test('should display sidebar', async () => {
    await expect(homePage.sidebar).toBeVisible();
  });

  test('should display image gallery or empty state', async () => {
    const hasImages = await homePage.hasImages();
    if (hasImages) {
      await expect(homePage.imageGallery).toBeVisible();
    } else {
      await expect(homePage.emptyState).toBeVisible();
    }
  });

  test('should display Advanced filter button', async () => {
    await expect(homePage.advancedButton).toBeVisible();
  });
});

test.describe('Home Page - Sidebar', () => {
  let homePage: HomePage;

  test.beforeEach(async ({ page }) => {
    homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();
  });

  test('should display Astrometry.net Status card', async () => {
    await expect(homePage.astrometryStatusCard).toBeVisible();
  });

  test('should display Recent Activity card', async () => {
    await expect(homePage.recentActivityCard).toBeVisible();
  });

  test('should display Popular Tags card', async () => {
    await expect(homePage.popularTagsCard).toBeVisible();
  });

  test('should display Submit New Image link in sidebar', async () => {
    await expect(homePage.submitNewImageLink).toBeVisible();
  });

  test('should navigate to plate solving from Submit New Image link', async () => {
    await homePage.submitNewImageLink.click();
    await expect(homePage.page).toHaveURL(/\/plate-solving/);
  });
});

test.describe('Home Page - Image Gallery', () => {
  let homePage: HomePage;

  test.beforeEach(async ({ page }) => {
    homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();
  });

  test('should display image cards with content when images exist', async () => {
    const hasImages = await homePage.hasImages();
    if (!hasImages) {
      test.skip();
      return;
    }

    const firstCard = homePage.imageCards.first();
    await expect(firstCard).toBeVisible();

    // Each card should have an image
    const img = firstCard.locator('img');
    await expect(img).toBeVisible();
  });

  test('should display plate solve status badge on image cards', async () => {
    const hasImages = await homePage.hasImages();
    if (!hasImages) {
      test.skip();
      return;
    }

    // Each card should have either "Plate Solved" or "No Plate Data" badge
    const firstCard = homePage.imageCards.first();
    const plateSolvedBadge = firstCard.getByText('Plate Solved');
    const noDataBadge = firstCard.getByText('No Plate Data');
    const hasBadge =
      (await plateSolvedBadge.count()) > 0 || (await noDataBadge.count()) > 0;
    expect(hasBadge).toBeTruthy();
  });

  test('should open image overlay when clicking an image card', async () => {
    const hasImages = await homePage.hasImages();
    if (!hasImages) {
      test.skip();
      return;
    }

    await homePage.clickFirstImage();

    // The overlay is a fixed inset-0 element with close button
    const closeButton = homePage.page.locator('button[aria-label="Close"]');
    await expect(closeButton).toBeVisible();
  });

  test('should close image overlay with close button', async () => {
    const hasImages = await homePage.hasImages();
    if (!hasImages) {
      test.skip();
      return;
    }

    await homePage.clickFirstImage();

    const closeButton = homePage.page.locator('button[aria-label="Close"]');
    await expect(closeButton).toBeVisible();
    await closeButton.click();

    // After closing, overlay should be gone
    await expect(closeButton).not.toBeVisible();
  });

  test('should display Load More button when more images exist', async () => {
    const hasImages = await homePage.hasImages();
    if (!hasImages) {
      test.skip();
      return;
    }

    const loadMore = homePage.loadMoreButton;
    const hasLoadMore = (await loadMore.count()) > 0;
    if (hasLoadMore) {
      await expect(loadMore).toBeVisible();
      // Button text should include count info
      const text = await loadMore.textContent();
      expect(text).toMatch(/load more images/i);
    }
  });
});

test.describe('Home Page - Image Overlay', () => {
  let homePage: HomePage;

  test.beforeEach(async ({ page }) => {
    homePage = new HomePage(page);
    await homePage.goto();
    await homePage.waitForLoad();

    const hasImages = await homePage.hasImages();
    if (!hasImages) {
      test.skip();
      return;
    }

    await homePage.clickFirstImage();
    await homePage.page.locator('button[aria-label="Close"]').waitFor();
  });

  test('should display close button', async () => {
    await expect(homePage.page.locator('button[aria-label="Close"]')).toBeVisible();
  });

  test('should display expand button', async () => {
    // Expand button is in the image viewer area
    const expandButton = homePage.page.getByRole('button', { name: /expand/i });
    if ((await expandButton.count()) > 0) {
      await expect(expandButton).toBeVisible();
    }
  });

  test('should display plate solution section', async () => {
    const plateSolutionText = homePage.page.getByText('Plate Solution');
    await expect(plateSolutionText).toBeVisible();
  });

  test('should display technical details section', async () => {
    const technicalDetails = homePage.page.getByText('Technical Details');
    await expect(technicalDetails).toBeVisible();
  });

  test('should display tags section', async () => {
    const tagsSection = homePage.page.getByText('Tags').first();
    await expect(tagsSection).toBeVisible();
  });
});

test.describe('Home Page - Deep Linking', () => {
  let homePage: HomePage;

  test.beforeEach(async ({ page }) => {
    homePage = new HomePage(page);
  });

  test('should open image overlay when navigating with ?image=ID', async ({ page }) => {
    // Assuming image with ID 1 exists in seed data
    await page.goto('/?image=1');
    await page.waitForLoadState('networkidle');

    // Verify overlay is open by checking for close button
    const closeButton = page.locator('button[aria-label="Close"]');
    await expect(closeButton).toBeVisible();

    // Verify correct image is loaded if possible, or at least that overlay is active
    await expect(page).toHaveURL(/\/.*image=1/);
  });

  test('should remove image parameter from URL when closing overlay', async ({ page }) => {
    await page.goto('/?image=1');
    await page.waitForLoadState('networkidle');

    const closeButton = page.locator('button[aria-label="Close"]');
    await expect(closeButton).toBeVisible();
    await closeButton.click();

    // After closing, image parameter should be gone from URL
    await expect(page).toHaveURL(/\/$/);
    await expect(closeButton).not.toBeVisible();
  });
});
