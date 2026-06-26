import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import mock from 'node:test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import sharp from 'sharp';
import { LocalUploadSource } from './local-upload.js';

describe('LocalUploadSource', () => {
  let tmpDir: string;

  before(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'sidereal-local-upload-test-'));
    process.env.STORAGE_PATH = tmpDir;
  });

  after(async () => {
    await rm(tmpDir, { recursive: true, force: true });
    delete process.env.STORAGE_PATH;
  });

  it('ingest writes image to storage and returns IngestResult', async () => {
    const bytes = await sharp({
      create: { width: 1, height: 1, channels: 3, background: { r: 255, g: 255, b: 255 } },
    }).jpeg().toBuffer();

    const mockStorage = {
      getImageBySource: async () => undefined,
      createAstroImage: async (img: Record<string, unknown>) => ({ ...img, id: 99, createdAt: new Date(), updatedAt: new Date() }),
      updateAstroImage: async () => undefined,
      deleteAstroImage: async () => undefined,
      getAstroImages: async () => [],
    };

    const source = new LocalUploadSource(mockStorage as never);

    const result = await source.ingest(bytes, 'test.jpg');

    assert.equal(result.imageId, 99);
    assert.equal(result.filename, 'test.jpg');
    assert.equal(result.sourceType, 'local');
    assert.ok(result.sourceId.length > 0, 'sourceId should be a hash');
  });

  it('ingest deduplicates — returns existing record if sourceId matches', async () => {
    const bytes = Buffer.from('fake-image-bytes');
    const existing = { id: 7, filename: 'existing.jpg', sourceType: 'local', sourceId: 'abc' };

    const mockStorage = {
      getImageBySource: async () => existing,
      createAstroImage: async () => { throw new Error('should not be called'); },
      updateAstroImage: async () => undefined,
      deleteAstroImage: async () => undefined,
      getAstroImages: async () => [],
    };

    const source = new LocalUploadSource(mockStorage as never);

    const result = await source.ingest(bytes, 'anything.jpg');

    assert.equal(result.imageId, 7);
    assert.equal(result.sourceType, 'local');
  });

  it('testConnection returns ok: true', async () => {
    const source = new LocalUploadSource({} as never);
    const result = await source.testConnection();
    assert.equal(result.ok, true);
  });

  it('ingest cleans up DB record if writeImage fails', async () => {
    const deleteAstroImage = mock.mock.fn(async () => {});

    const mockStorage = {
      getImageBySource: async () => undefined,
      createAstroImage: async (img: Record<string, unknown>) => ({ ...img, id: 42, createdAt: new Date(), updatedAt: new Date() }),
      updateAstroImage: async () => undefined,
      deleteAstroImage,
      getAstroImages: async () => [],
    };
    const mockImgStorage = {
      writeImage: async () => { throw new Error('disk full'); },
    };

    const source = new LocalUploadSource(mockStorage as never, mockImgStorage as never);

    await assert.rejects(() => source.ingest(Buffer.from('bytes'), 'test.jpg'), /disk full/);
    assert.equal(deleteAstroImage.mock.calls.length, 1, 'deleteAstroImage should be called');
  });
});
