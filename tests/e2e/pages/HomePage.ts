import { Page, Locator, expect } from '@playwright/test';
import { BasePage } from './BasePage';

export class HomePage extends BasePage {
  readonly searchInput: Locator;
  readonly imageGallery: Locator;
  readonly sidebar: Locator;
  readonly loadMoreButton: Locator;
  readonly appTitle: Locator;
  readonly imageCards: Locator;
  readonly objectTypeSelect: Locator;
  readonly plateSolvedSelect: Locator;
  readonly advancedButton: Locator;
  readonly emptyState: Locator;
  readonly githubLink: Locator;

  constructor(page: Page) {
    super(page);
    this.searchInput = page.getByPlaceholder('Search astrophotography images...');
    this.imageGallery = page.locator('main .grid').first();
    this.sidebar = page.locator('aside');
    this.loadMoreButton = page.getByRole('button', { name: /load more/i });
    this.appTitle = page.locator('header h1');
    this.imageCards = page.locator('main .grid .group');
    this.objectTypeSelect = page.getByRole('combobox').filter({ hasText: /all objects/i });
    this.plateSolvedSelect = page.getByRole('combobox').filter({ hasText: /all status/i });
    this.advancedButton = page.getByRole('button', { name: /advanced/i });
    this.emptyState = page.getByText(/no astrophotography images found/i);
    this.githubLink = page.locator('a[href="https://github.com/mstelz/sidereal"]');
  }

  async goto() {
    await super.goto('/');
  }

  async verifyPageElements() {
    await expect(this.header).toBeVisible();
    await expect(this.appTitle).toHaveText('Sidereal');
    await expect(this.searchInput).toBeVisible();
    await expect(this.syncButton).toBeVisible();
  }

  async verifyNavigation() {
    await expect(this.navigationLinks.gallery).toBeVisible();
    await expect(this.navigationLinks.equipment).toBeVisible();
    await expect(this.navigationLinks.plateSolving).toBeVisible();
    await expect(this.navigationLinks.skyMap).toBeVisible();
    await expect(this.navigationLinks.admin).toBeVisible();
  }

  async search(query: string) {
    await this.searchInput.fill(query);
    await this.searchInput.press('Enter');
  }

  async getImageCount() {
    return await this.imageCards.count();
  }

  async hasImages() {
    return (await this.imageCards.count()) > 0;
  }

  async clickFirstImage() {
    await this.imageCards.first().click();
  }

  // Sidebar card locators
  get astrometryStatusCard() {
    return this.page.getByText('Astrometry.net Status');
  }

  get recentActivityCard() {
    return this.page.getByText('Recent Activity', { exact: true });
  }

  get popularTagsCard() {
    return this.page.getByText('Popular Tags');
  }

  get submitNewImageLink() {
    return this.page.getByRole('link', { name: /submit new image/i });
  }
}
