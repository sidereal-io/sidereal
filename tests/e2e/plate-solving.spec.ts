import { test, expect, type Page } from '@playwright/test';
import { PlateSolvingPage } from './pages';

test.describe('Plate Solving Page', () => {
  let plateSolvingPage: PlateSolvingPage;

  test.beforeEach(async ({ page }) => {
    plateSolvingPage = new PlateSolvingPage(page);
    await plateSolvingPage.goto();
    await plateSolvingPage.waitForLoad();
  });

  test('should load with heading and description', async () => {
    await plateSolvingPage.verifyPageLoaded();
  });

  test('should display header with navigation', async () => {
    await expect(plateSolvingPage.header).toBeVisible();
    await expect(plateSolvingPage.syncButton).toBeVisible();
  });

  test('should display all five stats cards', async () => {
    await plateSolvingPage.verifyStatsCards();
  });

  test('should display stats cards with numeric values', async () => {
    // Each stat card should show a number (the count)
    const statsSection = plateSolvingPage.page.locator('.grid-cols-1');
    const boldNumbers = statsSection.locator('.text-2xl.font-bold');
    expect(await boldNumbers.count()).toBeGreaterThanOrEqual(5);
  });

  test('should display image selection card', async () => {
    await expect(plateSolvingPage.imageSelectionCard).toBeVisible();
  });

  test('should display search input for images', async () => {
    await expect(plateSolvingPage.searchInput).toBeVisible();
    await expect(plateSolvingPage.searchInput).toHaveAttribute('placeholder', 'Search images...');
  });

  test('should accept search input for filtering images', async () => {
    await plateSolvingPage.searchImages('M31');
    await expect(plateSolvingPage.searchInput).toHaveValue('M31');
  });

  test('should display show-only-unsolved checkbox', async () => {
    await expect(plateSolvingPage.showOnlyUnsolved).toBeVisible();
  });

  test('should display select all / deselect all button', async () => {
    await expect(plateSolvingPage.selectAllButton).toBeVisible();
  });

  test('should display image count text', async () => {
    const countText = plateSolvingPage.page.getByText(/\d+ of \d+ images selected/);
    if ((await countText.count()) > 0) {
      await expect(countText).toBeVisible();
    }
  });

  test('should display images or empty state', async () => {
    // Wait for loading to finish
    const loading = plateSolvingPage.loadingIndicator;
    if ((await loading.count()) > 0) {
      await expect(loading).not.toBeVisible({ timeout: 10000 });
    }

    const hasImages = (await plateSolvingPage.imageGrid.locator('.cursor-pointer').count()) > 0;
    if (!hasImages) {
      await expect(plateSolvingPage.emptyState).toBeVisible();
    }
  });
});


const screenshotDir = '/tmp/sidereal-309-screenshots';
const initialJobs = [
  { id: 14, imageId: 1, status: 'processing', astrometrySubmissionId: '16141301', astrometryJobId: 'null', submittedAt: '2026-10-06T12:00:00Z', completedAt: '2026-10-06T12:01:00Z', result: null },
  { id: 15, imageId: 2, status: 'processing', astrometrySubmissionId: '16141302', astrometryJobId: '123', submittedAt: '2026-10-06T12:00:00Z', completedAt: '2026-10-06T12:01:00Z', result: null },
  { id: 16, imageId: 3, status: 'failed', astrometrySubmissionId: '16141303', astrometryJobId: '124', submittedAt: '2026-10-06T12:00:00Z', completedAt: '2026-10-06T12:01:00Z', result: { error: 'Solver failed' } },
];
async function fixture(page: Page) {
  let jobs = structuredClone(initialJobs);
  const requests: string[] = [];
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname;
    requests.push(`${route.request().method()} ${path}`);
    const data = path === '/api/plate-solving/jobs' ? jobs : path === '/api/images' ? jobs.map((job, i) => ({ id: job.imageId, title: ['Waiting nebula', 'Solving galaxy', 'Failed cluster'][i], plateSolved: job.status === 'success', thumbnailUrl: null })) : path === '/api/stats' ? { totalImages: 3, plateSolved: jobs.filter(j => j.status === 'success').length, totalHours: 0, uniqueTargets: 3 } : [];
    await route.fulfill({ json: data });
  });
  return { requests, setJobs: (value: typeof jobs) => { jobs = value; }, jobs: () => jobs };
}

test.describe('Astrometry polling presentation', () => {
  test.use({ actionTimeout: 5000 });
  test('processing details validate remote IDs and hide nonterminal completion', async ({ page }) => {
    await fixture(page);
    const pom = new PlateSolvingPage(page);
    await pom.goto();
    await expect(pom.stage('Waiting nebula', 'Waiting for Astrometry.net')).toBeVisible();
    await pom.details('Waiting nebula').click();
    await expect(pom.submission('Waiting nebula')).toHaveAttribute('href', 'https://nova.astrometry.net/status/16141301');
    await expect(pom.completion('Waiting nebula')).toHaveCount(0);
    await expect(pom.record('Waiting nebula').getByRole('link', { name: 'Annotated Result' })).toHaveCount(0);
    await page.screenshot({ path: `${screenshotDir}/waiting-details.png`, fullPage: true });
    await expect(pom.stage('Solving galaxy', 'Solving')).toBeVisible();
    await pom.details('Solving galaxy').click();
    await expect(pom.record('Solving galaxy').getByRole('link', { name: 'Annotated Result' })).toHaveAttribute('href', 'https://nova.astrometry.net/annotated_full/123');
    await expect(pom.completion('Solving galaxy')).toHaveCount(0);
    await page.screenshot({ path: `${screenshotDir}/solving-details.png`, fullPage: true });
  });

  test('failed manual check restores processing on the same submission with stale error removed', async ({ page }) => {
    const state = await fixture(page);
    await page.route('**/api/plate-solving/update/16', async route => {
      state.requests.push('POST /api/plate-solving/update/16');
      state.setJobs(state.jobs().map(j => j.id === 16 ? { ...j, status: 'processing', result: null } : j));
      await route.fulfill({ json: { status: 'processing' } });
    });
    const pom = new PlateSolvingPage(page);
    await pom.goto();
    await pom.details('Failed cluster').click();
    await expect(pom.record('Failed cluster').getByText('Solver failed', { exact: true })).toBeVisible();
    await pom.checkStatus('Failed cluster').click();
    await expect(pom.stage('Failed cluster', 'Solving')).toBeVisible();
    await expect(pom.record('Failed cluster').getByText('Solver failed', { exact: true })).toHaveCount(0);
    await expect(pom.submission('Failed cluster')).toHaveAttribute('href', 'https://nova.astrometry.net/status/16141303');
    await expect(pom.completion('Failed cluster')).toHaveCount(0);
    expect(state.requests.filter(r => r.startsWith('POST'))).toEqual(['POST /api/plate-solving/update/16']);
  });

  test('checks are per record and preserve details while processing and on request errors', async ({ page }) => {
    const state = await fixture(page);
    let release!: () => void;
    const pending = new Promise<void>(resolve => { release = resolve; });
    let checks = 0;
    await page.route('**/api/plate-solving/update/14', async route => {
      checks++;
      state.requests.push('POST /api/plate-solving/update/14');
      await pending;
      await route.fulfill({ json: { status: 'processing' } });
    });
    await page.route('**/api/plate-solving/update/16', route => { state.requests.push('POST /api/plate-solving/update/16'); return route.fulfill({ status: 503, json: { error: 'Upstream unavailable' } }); });
    const pom = new PlateSolvingPage(page);
    await pom.goto();
    await pom.details('Waiting nebula').click();
    await pom.checkStatus('Waiting nebula').click();
    await expect(pom.checking('Waiting nebula')).toBeDisabled();
    await pom.checking('Waiting nebula').evaluate((button: HTMLButtonElement) => button.click());
    expect(checks).toBe(1);
    await expect(pom.checkStatus('Solving galaxy')).toBeEnabled();
    await page.screenshot({ path: `${screenshotDir}/checking-record.png`, fullPage: true });
    release();
    await expect(pom.checkStatus('Waiting nebula')).toBeEnabled();
    await expect(pom.submission('Waiting nebula')).toBeVisible();
    await pom.details('Failed cluster').click();
    await pom.checkStatus('Failed cluster').click();
    await expect(pom.record('Failed cluster').getByText('Couldn’t check status. Try again.')).toBeVisible();
    await expect(pom.record('Failed cluster').getByText('Solver failed', { exact: true })).toBeVisible();
    await expect(pom.checkStatus('Failed cluster')).toBeEnabled();
    await page.screenshot({ path: `${screenshotDir}/request-error-preserved-failure.png`, fullPage: true });
    expect(state.requests.filter(r => r.startsWith('POST'))).toEqual(['POST /api/plate-solving/update/14', 'POST /api/plate-solving/update/16']);
  });

  test('manual recovery refreshes jobs images and stats and retains explicit failures', async ({ page }) => {
    const state = await fixture(page);
    await page.route('**/api/plate-solving/update/16', async route => {
      state.setJobs(state.jobs().map(j => j.id === 16 ? { ...j, status: 'success', result: { error: '' } } : j));
      await route.fulfill({ json: { status: 'success' } });
    });
    await page.route('**/api/plate-solving/update/15', async route => {
      state.setJobs(state.jobs().map(j => j.id === 15 ? { ...j, status: 'failed', result: { error: 'Explicit upstream failure' } } : j));
      await route.fulfill({ json: { status: 'failed' } });
    });
    const pom = new PlateSolvingPage(page);
    await pom.goto();
    await page.evaluate(async () => {
      const { queryClient } = await import('/src/lib/queryClient.ts');
      await queryClient.fetchQuery({ queryKey: ['/api/stats'] });
    });
    await pom.showOnlyUnsolved.uncheck();
    await pom.checkStatus('Failed cluster').click();
    await expect(pom.record('Failed cluster').getByText('Solved', { exact: true }).first()).toBeVisible();
    await expect.poll(() => state.requests.filter(r => r === 'GET /api/images').length).toBeGreaterThan(1);
    expect(await page.evaluate(async () => {
      const { queryClient } = await import('/src/lib/queryClient.ts');
      return queryClient.getQueryState(['/api/stats'])?.isInvalidated;
    })).toBe(true);
    await pom.checkStatus('Solving galaxy').click();
    await pom.details('Solving galaxy').click();
    await expect(pom.record('Solving galaxy').getByText('Explicit upstream failure')).toBeVisible();
    await expect(pom.completion('Solving galaxy')).toBeVisible();
  });

  test('30 second read polling observes completion and stops at terminal jobs; focus recovers stale data', async ({ page }) => {
    await page.clock.install();
    const state = await fixture(page);
    const pom = new PlateSolvingPage(page);
    await pom.goto();
    await expect(pom.record('Waiting nebula')).toBeVisible();
    await pom.showOnlyUnsolved.uncheck();
    const reads = () => state.requests.filter(r => r === 'GET /api/plate-solving/jobs').length;
    const start = reads();
    await page.clock.runFor(30_001);
    await expect.poll(reads).toBeGreaterThan(start);
    state.setJobs(state.jobs().map(j => ({ ...j, status: 'success' })));
    const imagesBefore = state.requests.filter(r => r === 'GET /api/images').length;
    await page.clock.runFor(30_001);
    await expect.poll(() => state.requests.filter(r => r === 'GET /api/images').length).toBeGreaterThan(imagesBefore);
    const terminalReads = reads();
    await page.clock.runFor(60_001);
    expect(reads()).toBe(terminalReads);
    state.setJobs(state.jobs().map(j => j.id === 14 ? { ...j, status: 'processing' } : j));
    await page.evaluate(() => { window.dispatchEvent(new Event('visibilitychange')); });
    // TanStack's focus manager listens to visibilitychange, even for fresh cached data.
    await expect.poll(reads).toBeGreaterThan(terminalReads);
    await expect(pom.stage('Waiting nebula', 'Waiting for Astrometry.net')).toBeVisible();
    const focusImagesBefore = state.requests.filter(r => r === 'GET /api/images').length;
    state.setJobs(state.jobs().map(j => ({ ...j, status: 'success' })));
    await page.evaluate(() => { window.dispatchEvent(new Event('visibilitychange')); });
    await expect(pom.record('Waiting nebula').getByText('Solved', { exact: true }).last()).toBeVisible();
    await expect.poll(() => state.requests.filter(r => r === 'GET /api/images').length).toBeGreaterThan(focusImagesBefore);
    expect(state.requests.filter(r => r.startsWith('POST'))).toEqual([]);
  });

  test('actual ImageModal handles processing and refreshes images and stats on successful solve', async ({ page }) => {
    const state = await fixture(page);
    const posts: string[] = [];
    let processing = true;
    await page.route('**/api/**/plate-solve', async route => {
      posts.push(new URL(route.request().url()).pathname);
      if (processing) {
        await route.fulfill({ status: 202, json: { status: 'processing', message: 'Still processing. Check status later.', jobId: 14, submissionId: '16141301' } });
      } else {
        state.setJobs(state.jobs().map(job => job.id === 14 ? { ...job, status: 'success' } : job));
        await route.fulfill({ status: 200, json: {
          message: 'Image plate solving completed successfully',
          result: {
            calibration: { ra: 0, dec: 0, pixscale: 1, radius: 2, orientation: 0 },
            annotations: [],
            machineTags: [],
          },
        } });
      }
    });
    await page.goto('/');
    await page.evaluate(async () => {
      // Render the actual otherwise-unmounted component through Vite, without a product fixture route.
      const mainSource = await (await fetch('/src/main.tsx')).text();
      const modalSource = await (await fetch('/src/components/image-modal.tsx')).text();
      const dependency = (source: string, name: string) => {
        const url = [...source.matchAll(/from\s+["']([^"']+)["']/g)].map(match => match[1]).find(url => url.includes(`/${name}.js`));
        if (!url) throw new Error(`Missing Vite dependency ${name}`);
        return url;
      };
      const reactModule = await import(dependency(modalSource, 'react'));
      const React = reactModule.default ?? reactModule;
      const domModule = await import(dependency(mainSource, 'react-dom_client'));
      const { createRoot } = domModule.default ?? domModule;
      const { QueryClientProvider } = await import(dependency(modalSource, '@tanstack_react-query'));
      const { queryClient } = await import('/src/lib/queryClient.ts');
      const { ImageModal } = await import('/src/components/image-modal.tsx');
      const root = document.createElement('div'); document.body.append(root);
      createRoot(root).render(React.createElement(QueryClientProvider, { client: queryClient }, React.createElement(ImageModal, { image: { id: 1, title: 'Waiting nebula', plateSolved: false }, onClose: () => {} })));
    });
    const reads = (path: string) => state.requests.filter(request => request === `GET ${path}`).length;
    const initialImages = reads('/api/images');
    const initialStats = reads('/api/stats');
    await page.getByRole('button', { name: 'Solve', exact: true }).click();
    await expect(page.getByText('Still processing. Check status later.', { exact: true })).toBeVisible();
    await expect.poll(() => reads('/api/images')).toBeGreaterThan(initialImages);
    await expect.poll(() => reads('/api/stats')).toBeGreaterThan(initialStats);
    expect(posts).toEqual(['/api/plate-solving/images/1/plate-solve']);
    await expect(page.getByText('Click "Solve" to submit this image to Astrometry.net for plate solving.', { exact: true })).toHaveCount(0);
    await page.screenshot({ path: `${screenshotDir}/image-modal-still-processing-component-fixture.png`, fullPage: true });
    processing = false;
    const processingImages = reads('/api/images');
    const processingStats = reads('/api/stats');
    await page.getByRole('button', { name: 'Solve', exact: true }).click();
    await expect(page.getByText('Still processing. Check status later.', { exact: true })).toHaveCount(0);
    await expect.poll(() => reads('/api/images')).toBeGreaterThan(processingImages);
    await expect.poll(() => reads('/api/stats')).toBeGreaterThan(processingStats);
    expect(posts).toEqual(['/api/plate-solving/images/1/plate-solve', '/api/plate-solving/images/1/plate-solve']);
  });
});
