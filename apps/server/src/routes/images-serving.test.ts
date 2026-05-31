import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// Set STORAGE_PATH before any module imports
let tmpDir: string;

describe('image serving routes', () => {
  before(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'sidereal-routes-test-'));
    process.env.STORAGE_PATH = tmpDir;

    // Pre-write a test image for ID 1 using image-storage
    const { imageStorage } = await import('../services/image-storage.js');
    const { default: sharp } = await import('sharp');
    const buf = await sharp({
      create: { width: 10, height: 10, channels: 3, background: { r: 100, g: 150, b: 200 } }
    }).jpeg().toBuffer();
    await imageStorage.writeImage(1, buf, 'jpg');
  });

  after(async () => {
    await rm(tmpDir, { recursive: true, force: true });
    delete process.env.STORAGE_PATH;
  });

  async function buildApp() {
    const { default: imageRoutes } = await import('./images.js');
    const { Hono } = await import('hono');
    const app = new Hono();
    app.route('/', imageRoutes);
    return app;
  }

  it('GET /:id/thumbnail returns 200 with image/jpeg content-type', async () => {
    const app = await buildApp();
    const res = await app.request('/1/thumbnail');
    assert.equal(res.status, 200);
    assert.ok(res.headers.get('content-type')?.includes('image/jpeg'));
    assert.ok(res.headers.get('cache-control')?.includes('max-age=31536000'));
  });

  it('GET /:id/preview returns 200', async () => {
    const app = await buildApp();
    const res = await app.request('/1/preview');
    assert.equal(res.status, 200);
  });

  it('GET /:id/thumbnail returns 404 for unknown image', async () => {
    const app = await buildApp();
    const res = await app.request('/99999/thumbnail');
    assert.equal(res.status, 404);
  });

  it('GET /:id/original with Range header returns 206', async () => {
    const app = await buildApp();
    const res = await app.request('/1/original', {
      headers: { Range: 'bytes=0-99' }
    });
    assert.equal(res.status, 206);
    assert.ok(res.headers.get('content-range')?.startsWith('bytes 0-'));
  });

  it('GET /:id/original with invalid Range returns 416', async () => {
    const app = await buildApp();
    const res = await app.request('/1/original', {
      headers: { Range: 'bytes=9999999-9999999' }
    });
    assert.equal(res.status, 416);
  });

  it('GET /:id/thumbnail with matching ETag returns 304', async () => {
    const app = await buildApp();
    // First request to get ETag
    const first = await app.request('/1/thumbnail');
    const etag = first.headers.get('etag');
    assert.ok(etag, 'ETag must be present');

    // Second request with If-None-Match
    const second = await app.request('/1/thumbnail', {
      headers: { 'If-None-Match': etag! }
    });
    assert.equal(second.status, 304);
  });
});
