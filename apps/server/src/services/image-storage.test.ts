import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm, readdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// Set env before any import of image-storage
let tmpDir: string;

describe('image-storage', () => {
  before(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'sidereal-storage-test-'));
    process.env.STORAGE_PATH = tmpDir;
  });

  after(async () => {
    await rm(tmpDir, { recursive: true, force: true });
    delete process.env.STORAGE_PATH;
  });

  it('writeImage creates original, preview, and thumbnail files', async () => {
    // Lazy import so STORAGE_PATH env is set first
    const { imageStorage } = await import('./image-storage.js');
    // 1x1 white JPEG — minimal valid image
    const { default: sharp } = await import('sharp');
    const buf = await sharp({ create: { width: 1, height: 1, channels: 3, background: { r: 255, g: 255, b: 255 } } })
      .jpeg().toBuffer();

    await imageStorage.writeImage(42, buf, 'jpg');

    const dir = join(tmpDir, 'processed', '042', '42');
    const files = await readdir(dir);
    assert.ok(files.some(f => f === '42_original.jpg'), 'original missing');
    assert.ok(files.some(f => f === '42_preview.jpg'), 'preview missing');
    assert.ok(files.some(f => f === '42_thumb.jpg'), 'thumb missing');
  });

  it('readPath returns correct paths after writeImage', async () => {
    const { imageStorage } = await import('./image-storage.js');
    const { default: sharp } = await import('sharp');
    const buf = await sharp({ create: { width: 100, height: 100, channels: 3, background: { r: 0, g: 0, b: 0 } } })
      .jpeg().toBuffer();

    await imageStorage.writeImage(99, buf, 'jpg');

    const origPath = await imageStorage.readPath(99, 'original');
    const prevPath = await imageStorage.readPath(99, 'preview');
    const thumbPath = await imageStorage.readPath(99, 'thumbnail');

    assert.ok(origPath.endsWith('99_original.jpg'));
    assert.ok(prevPath.endsWith('99_preview.jpg'));
    assert.ok(thumbPath.endsWith('99_thumb.jpg'));
  });

  it('readPath throws ImageNotFoundError for missing image', async () => {
    const { imageStorage, ImageNotFoundError } = await import('./image-storage.js');
    await assert.rejects(
      () => imageStorage.readPath(99999, 'thumbnail'),
      ImageNotFoundError
    );
  });

  it('imageExists returns true after write, false before', async () => {
    const { imageStorage } = await import('./image-storage.js');
    const { default: sharp } = await import('sharp');
    const buf = await sharp({ create: { width: 1, height: 1, channels: 3, background: { r: 128, g: 128, b: 128 } } })
      .jpeg().toBuffer();

    assert.equal(await imageStorage.imageExists(777, 'thumbnail'), false);
    await imageStorage.writeImage(777, buf, 'jpg');
    assert.equal(await imageStorage.imageExists(777, 'thumbnail'), true);
  });

  it('deleteImage removes the per-image directory', async () => {
    const { imageStorage } = await import('./image-storage.js');
    const { default: sharp } = await import('sharp');
    const buf = await sharp({ create: { width: 1, height: 1, channels: 3, background: { r: 0, g: 0, b: 255 } } })
      .jpeg().toBuffer();

    await imageStorage.writeImage(555, buf, 'jpg');
    assert.equal(await imageStorage.imageExists(555, 'thumbnail'), true);

    await imageStorage.deleteImage(555);
    assert.equal(await imageStorage.imageExists(555, 'thumbnail'), false);
  });

  it('shard buckets correctly: id 0 → 000, id 999 → 999, id 1000 → 000', async () => {
    const { shardBucket } = await import('./image-storage.js');
    assert.equal(shardBucket(0), '000');
    assert.equal(shardBucket(999), '999');
    assert.equal(shardBucket(1000), '000');
    assert.equal(shardBucket(42), '042');
  });
});
