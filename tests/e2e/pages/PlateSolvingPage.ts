import { Page, Locator, expect } from '@playwright/test';
import { BasePage } from './BasePage';

export class PlateSolvingPage extends BasePage {
  readonly heading: Locator;
  readonly description: Locator;
  readonly searchInput: Locator;
  readonly showOnlyUnsolved: Locator;
  readonly imageSelectionCard: Locator;
  readonly selectAllButton: Locator;
  readonly imageGrid: Locator;
  readonly loadingIndicator: Locator;
  readonly emptyState: Locator;

  constructor(page: Page) {
    super(page);
    this.heading = page.getByRole('heading', { name: /plate solving/i, level: 1 });
    this.description = page.getByText('Manually submit images for plate solving using Astrometry.net');
    this.searchInput = page.locator('#search');
    this.showOnlyUnsolved = page.locator('#showOnlyUnsolved');
    this.imageSelectionCard = page.getByText('Image Selection');
    this.selectAllButton = page.getByRole('button', { name: /select all|deselect all/i });
    this.imageGrid = page.locator('.grid-cols-2');
    this.loadingIndicator = page.getByText(/loading images/i);
    this.emptyState = page.getByText(/no images found/i);
  }

  record(title: string) {
    return this.imageGrid.locator('.cursor-pointer').filter({ has: this.page.getByRole('heading', { name: title, exact: true }) });
  }

  stage(title: string, stage: string) { return this.record(title).getByText(stage, { exact: true }); }
  details(title: string) { return this.record(title).getByRole('button', { name: 'Details', exact: true }); }
  submission(title: string) { return this.record(title).getByRole('link', { name: 'Submission', exact: true }); }
  completion(title: string) { return this.record(title).getByText(/^Completed:/); }
  checkStatus(title: string) { return this.record(title).getByRole('button', { name: 'Check status', exact: true }); }
  checking(title: string) { return this.record(title).getByRole('button', { name: 'Checking…', exact: true }); }

  async goto() {
    await super.goto('/plate-solving');
  }

  async verifyPageLoaded() {
    await expect(this.page).toHaveURL(/\/plate-solving/);
    await expect(this.heading).toBeVisible();
    await expect(this.description).toBeVisible();
  }

  async verifyStatsCards() {
    await expect(this.page.getByText('Total Jobs')).toBeVisible();
    await expect(this.page.getByText('Pending', { exact: true })).toBeVisible();
    await expect(this.page.getByText('Processing', { exact: true })).toBeVisible();
    await expect(this.page.getByText('Completed', { exact: true })).toBeVisible();
    await expect(this.page.getByText('Failed', { exact: true })).toBeVisible();
  }

  async searchImages(query: string) {
    await this.searchInput.fill(query);
  }

  async getSelectedCount() {
    const text = await this.page.getByText(/\d+ of \d+ images selected/).textContent();
    const match = text?.match(/(\d+) of (\d+)/);
    return match ? { selected: parseInt(match[1]), total: parseInt(match[2]) } : null;
  }
}
