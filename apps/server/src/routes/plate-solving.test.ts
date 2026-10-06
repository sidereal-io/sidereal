import { it } from 'node:test';
import assert from 'node:assert/strict';
import { Hono } from 'hono';
delete process.env.DATABASE_URL;
process.env.SQLITE_DB_PATH = ':memory:';
const { default: routes } = await import('./plate-solving');
const { astrometryService } = await import('../services/astrometry');
const { storage } = await import('../services/storage');
const { configService } = await import('../services/config');

for (const status of ['processing', 'success', 'failed']) {
  it(`single-image route preserves ${status} response`, async t => {
    t.mock.method(console, 'error', () => {});
    t.mock.method(configService, 'getAstrometryConfig', async () => ({ enabled: true, apiKey: 'test' }));
    t.mock.method(storage, 'getAstroImage', async () => ({ id: 1 }));
    const result = { calibration: { ra: 1, dec: 2, pixscale: 3, radius: 4, orientation: 5 }, annotations: [], machineTags: [] };
    const processing = { status: 'processing', message: 'Still processing. Check status later.', jobId: 14, submissionId: '16141301' };
    t.mock.method(astrometryService, 'completePlateSolvingWorkflow', async image => {
      assert.equal(image.id, 1);
      if (status === 'failed') throw new Error('bad image');
      return status === 'success' ? { status, result } : processing;
    });
    const app = new Hono().route('/api/plate-solving', routes());
    const response = await app.request('/api/plate-solving/images/1/plate-solve', { method: 'POST' });
    assert.equal(response.status, status === 'processing' ? 202 : status === 'success' ? 200 : 500);
    assert.deepEqual(await response.json(), status === 'processing' ? processing : status === 'success' ? { message: 'Image plate solving completed successfully', result } : { message: 'Failed to complete plate solving' });
  });
}

it('update route opts into recovery using existing local ID without submission', async t => {
  let options;
  t.mock.method(astrometryService, 'checkJobStatus', async (id, opts) => { assert.equal(id, 14); options = opts; return { status: 'processing' }; });
  t.mock.method(astrometryService, 'submitImageForPlateSolving', async () => assert.fail('No upload'));
  t.mock.method(storage, 'createPlateSolvingJob', async () => assert.fail('No new job'));
  const app = new Hono().route('/api/plate-solving', routes());
  const response = await app.request('/api/plate-solving/update/14', { method: 'POST' });
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { status: 'processing' });
  assert.deepEqual(options, { resumeProcessing: true });
});
