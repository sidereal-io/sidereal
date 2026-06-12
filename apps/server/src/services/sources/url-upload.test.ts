import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import mock from 'node:test';
import sharp from 'sharp';
import { UrlUploadSource } from './url-upload.js';

describe('UrlUploadSource', () => {
  it('ingest fetches URL and writes to storage', async () => {
    const bytes = await sharp({
      create: { width: 1, height: 1, channels: 3, background: { r: 0, g: 0, b: 0 } },
    }).jpeg().toBuffer();

    const mockFetch = mock.mock.fn(async (_url: string) => ({
      ok: true,
      arrayBuffer: async () => bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength),
      headers: { get: (_h: string) => 'image/jpeg' },
    }));

    const mockStorage = {
      getImageBySource: async () => undefined,
      createAstroImage: async (img: Record<string, unknown>) => ({ ...img, id: 55, createdAt: new Date(), updatedAt: new Date() }),
      updateAstroImage: async () => undefined,
      deleteAstroImage: async () => undefined,
      getAstroImages: async () => [],
    };

    const mockImgStorage = {
      writeImage: async (_id: number, _bytes: Buffer, _ext: string) => ({ originalPath: '/data/images/55.jpg' }),
    };

    const source = new UrlUploadSource(mockStorage as never, mockFetch as never, mockImgStorage as never);

    const result = await source.ingest('https://example.com/photo.jpg');

    assert.equal(result.imageId, 55);
    assert.equal(result.sourceType, 'url');
    assert.ok(result.sourceId.length > 0, 'sourceId should be a hash');
    assert.ok(result.filename.length > 0, 'filename should be set');
  });

  it('ingest deduplicates on same URL', async () => {
    const existing = { id: 7, filename: 'photo.jpg', sourceType: 'url', sourceId: 'abc123' };

    const mockFetch = mock.mock.fn(async (_url: string) => {
      throw new Error('should not be called');
    });

    const mockStorage = {
      getImageBySource: async () => existing,
      createAstroImage: async () => { throw new Error('should not be called'); },
      updateAstroImage: async () => undefined,
      deleteAstroImage: async () => undefined,
      getAstroImages: async () => [],
    };

    const source = new UrlUploadSource(mockStorage as never, mockFetch as never);

    const result = await source.ingest('https://example.com/photo.jpg');

    assert.equal(result.imageId, 7);
    assert.equal(result.sourceType, 'url');
    assert.equal(mockFetch.mock.calls.length, 0, 'fetch should not be called');
  });

  it('ingest rejects non-http URLs', async () => {
    const source = new UrlUploadSource({} as never, undefined, undefined);

    await assert.rejects(
      () => source.ingest('file:///etc/passwd'),
      /only http/i,
    );
  });

  it('ingest throws if fetch fails', async () => {
    const mockFetch = mock.mock.fn(async (_url: string) => ({
      ok: false,
      status: 404,
      statusText: 'Not Found',
      arrayBuffer: async () => new ArrayBuffer(0),
      headers: { get: (_h: string) => null },
    }));

    const mockStorage = {
      getImageBySource: async () => undefined,
      createAstroImage: async () => { throw new Error('should not be called'); },
      updateAstroImage: async () => undefined,
      deleteAstroImage: async () => undefined,
      getAstroImages: async () => [],
    };

    const source = new UrlUploadSource(mockStorage as never, mockFetch as never);

    await assert.rejects(
      () => source.ingest('https://example.com/missing.jpg'),
      /404/,
    );
  });

  it('ingest cleans up DB record if writeImage fails', async () => {
    const bytes = await sharp({
      create: { width: 1, height: 1, channels: 3, background: { r: 0, g: 0, b: 0 } },
    }).jpeg().toBuffer();

    const mockFetch = mock.mock.fn(async (_url: string) => ({
      ok: true,
      arrayBuffer: async () => bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength),
      headers: { get: (_h: string) => 'image/jpeg' },
    }));

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

    const source = new UrlUploadSource(mockStorage as never, mockFetch as never, mockImgStorage as never);

    await assert.rejects(
      () => source.ingest('https://example.com/photo.jpg'),
      /disk full/,
    );
    assert.equal(deleteAstroImage.mock.calls.length, 1, 'deleteAstroImage should be called');
  });
});
