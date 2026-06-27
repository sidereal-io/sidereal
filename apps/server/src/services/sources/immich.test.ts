import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import mock from 'node:test';
import { ImmichImageSource } from './immich.js';

type Row = { id: number; sourceType: string; sourceId: string; filename: string; originalPath: string | null };

function makeDb(initial: Row[] = []) {
  const rows = [...initial];
  let nextId = (initial.at(-1)?.id ?? 0) + 1;
  return {
    rows,
    getImageBySource: mock.mock.fn(async (st: string, sid: string) => rows.find(r => r.sourceType === st && r.sourceId === sid)),
    getAstroImages: mock.mock.fn(async () => rows),
    createAstroImage: mock.mock.fn(async (img: Record<string, unknown>) => { const r = { ...img, id: nextId++ }; rows.push(r as Row); return r; }),
    updateAstroImage: mock.mock.fn(async (id: number, patch: Record<string, unknown>) => { Object.assign(rows.find(r => r.id === id)!, patch); }),
    deleteAstroImage: mock.mock.fn(async (id: number) => { const i = rows.findIndex(r => r.id === id); if (i >= 0) rows.splice(i, 1); }),
  };
}

const cfg = { getImmichConfig: mock.mock.fn(async () => ({ host: 'http://immich.test', apiKey: 'k', syncByAlbum: false, selectedAlbumIds: [] as string[] })) };

function jsonResponse(body: unknown) {
  return { ok: true, status: 200, json: async () => body, arrayBuffer: async () => new ArrayBuffer(0), headers: { get: () => null } };
}
function bytesResponse(buf: ArrayBuffer, filename = 'a.fit') {
  return { ok: true, status: 200, json: async () => ({}), arrayBuffer: async () => buf, headers: { get: (n: string) => n.toLowerCase() === 'content-disposition' ? `attachment; filename="${filename}"` : null } };
}

describe('ImmichImageSource.sync', () => {
  it('creates new images with sourceType/sourceId and dedups', async () => {
    const db = makeDb();
    const imgStorage = { writeImage: mock.mock.fn(async (id: number) => ({ originalPath: `/p/${id}.fit` })) };
    const fetchFn = mock.mock.fn(async (url: string) => {
      if (url.includes('/api/search/metadata')) return jsonResponse({ assets: { items: [{ id: 'a1', originalFileName: 'a1.fit', exifInfo: {} }], nextPage: null } });
      if (url.includes('/api/assets/')) return bytesResponse(new Uint8Array([1, 2, 3]).buffer);
      return jsonResponse({});
    });
    const src = new ImmichImageSource(db as never, imgStorage as never, cfg as never, fetchFn as never);

    const r1 = await src.sync();
    assert.equal(r1.syncedCount, 1, 'first sync should create 1 image');
    assert.equal(db.rows[0]?.sourceType, 'immich', 'sourceType should be immich');
    assert.equal(db.rows[0]?.sourceId, 'a1', 'sourceId should be a1');
    assert.equal(db.rows[0]?.originalPath, '/p/1.fit', 'originalPath should be set');

    const r2 = await src.sync(); // second pass: dedup, nothing new
    assert.equal(r2.syncedCount, 0, 'second sync should create nothing (dedup)');
  });

  it('rolls back the DB row when writeImage fails', async () => {
    const db = makeDb();
    const imgStorage = { writeImage: mock.mock.fn(async () => { throw new Error('disk full'); }) };
    const fetchFn = mock.mock.fn(async (url: string) => {
      if (url.includes('/api/search/metadata')) return jsonResponse({ assets: { items: [{ id: 'a1', originalFileName: 'a1.fit', exifInfo: {} }], nextPage: null } });
      if (url.includes('/api/assets/')) return bytesResponse(new Uint8Array([1]).buffer);
      return jsonResponse({});
    });
    const src = new ImmichImageSource(db as never, imgStorage as never, cfg as never, fetchFn as never);
    const r = await src.sync();
    assert.equal(r.syncedCount, 0, 'failed sync should have syncedCount 0');
    assert.equal(db.rows.length, 0, 'DB row should be rolled back after writeImage failure');
  });

  it('removes images absent from the Immich response', async () => {
    const db = makeDb([{ id: 1, sourceType: 'immich', sourceId: 'gone', filename: 'x', originalPath: '/p/1' }]);
    const imgStorage = { writeImage: mock.mock.fn(async (id: number) => ({ originalPath: `/p/${id}` })) };
    const fetchFn = mock.mock.fn(async (url: string) => url.includes('/api/search/metadata') ? jsonResponse({ assets: { items: [], nextPage: null } }) : jsonResponse({}));
    const src = new ImmichImageSource(db as never, imgStorage as never, cfg as never, fetchFn as never);
    const r = await src.sync();
    assert.equal(r.removedCount, 1, 'should have removed 1 absent image');
    assert.equal(db.rows.length, 0, 'DB should be empty after removal');
  });
});

describe('ImmichImageSource.testConnection', () => {
  it('returns ok:false on non-200', async () => {
    const fetchFn = mock.mock.fn(async () => ({ ok: false, status: 401, json: async () => ({}), arrayBuffer: async () => new ArrayBuffer(0), headers: { get: () => null } }));
    const src = new ImmichImageSource(makeDb() as never, { writeImage: mock.mock.fn() } as never, cfg as never, fetchFn as never);
    const result = await src.testConnection();
    assert.equal(result.ok, false, 'should return ok:false');
    assert.equal(result.message, 'Immich returned status 401', 'should include status in message');
  });

  it('returns ok:true on 200', async () => {
    const fetchFn = mock.mock.fn(async () => jsonResponse([]));
    const src = new ImmichImageSource(makeDb() as never, { writeImage: mock.mock.fn() } as never, cfg as never, fetchFn as never);
    assert.deepEqual(await src.testConnection(), { ok: true });
  });
});

describe('ImmichImageSource.getStatus', () => {
  it('counts only immich-sourced images', async () => {
    const db = makeDb([
      { id: 1, sourceType: 'immich', sourceId: 'a1', filename: 'x', originalPath: '/p/1' },
      { id: 2, sourceType: 'local',  sourceId: 'l1', filename: 'y', originalPath: '/p/2' },
      { id: 3, sourceType: 'immich', sourceId: 'a2', filename: 'z', originalPath: '/p/3' },
    ]);
    const src = new ImmichImageSource(db as never, { writeImage: mock.mock.fn() } as never, cfg as never, mock.mock.fn() as never);
    const status = await src.getStatus();
    assert.equal(status.sourceType, 'immich');
    assert.equal(status.imageCount, 2);
  });
});

describe('ImmichImageSource.sync (album mode)', () => {
  it('ingests assets from selected albums', async () => {
    const albumCfg = { getImmichConfig: mock.mock.fn(async () => ({ host: 'http://immich.test', apiKey: 'k', syncByAlbum: true, selectedAlbumIds: ['alb1'] })) };
    const db = makeDb();
    const imgStorage = { writeImage: mock.mock.fn(async (id: number) => ({ originalPath: `/p/${id}.fit` })) };
    const fetchFn = mock.mock.fn(async (url: string) => {
      if (url === 'http://immich.test/api/albums') return jsonResponse([{ id: 'alb1', albumName: 'A', assetCount: 1 }]);
      if (url === 'http://immich.test/api/albums/alb1') return jsonResponse({ assets: [{ id: 'a1', originalFileName: 'a1.fit', exifInfo: {} }] });
      if (url.includes('/api/assets/')) return bytesResponse(new Uint8Array([1, 2, 3]).buffer);
      return jsonResponse({});
    });
    const src = new ImmichImageSource(db as never, imgStorage as never, albumCfg as never, fetchFn as never);
    const r = await src.sync();
    assert.equal(r.syncedCount, 1);
    assert.equal(db.rows[0]?.sourceType, 'immich');
    assert.equal(db.rows[0]?.sourceId, 'a1');
  });
});
