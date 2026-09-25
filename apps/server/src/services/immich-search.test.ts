import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { searchImmichAssets } from './immich-search';

const config = { host: 'http://immich.test', apiKey: 'k' };

function fakeImmich(pages: Record<string, unknown>[][]) {
  const bodies: Record<string, unknown>[] = [];
  const calls: string[] = [];
  const fetchFn = async (url: string, init: { body: string }) => {
    calls.push(url);
    const body = JSON.parse(init.body) as Record<string, unknown>;
    bodies.push(body);
    const index = Number(body.page) - 1;
    const nextPage = index + 1 < pages.length ? String(index + 2) : null;
    return { json: async () => ({ assets: { items: pages[index] ?? [], nextPage } }) };
  };
  return { fetchFn, bodies, calls };
}

describe('searchImmichAssets', () => {
  it('follows string nextPage values until the last page', async () => {
    const immich = fakeImmich([[{ id: 'a1' }], [{ id: 'a2' }], [{ id: 'a3' }]]);
    const assets = await searchImmichAssets(config, {}, immich.fetchFn);
    assert.deepEqual(assets.map((a) => a.id), ['a1', 'a2', 'a3']);
    assert.deepEqual(immich.bodies.map((b) => b.page), [1, 2, 3], 'page must be sent as a number');
    assert.ok(immich.calls.every((url) => url === 'http://immich.test/api/search/metadata'));
  });

  it('scopes the search to an album when given albumIds', async () => {
    const immich = fakeImmich([[{ id: 'a1' }, { id: 'a2' }]]);
    const assets = await searchImmichAssets(config, { albumIds: ['alb1'] }, immich.fetchFn);
    assert.equal(assets.length, 2);
    assert.deepEqual(immich.bodies[0], { albumIds: ['alb1'], size: 1000, page: 1, type: 'IMAGE' });
  });

  it('stops on an empty page even if nextPage is set', async () => {
    const bodies: unknown[] = [];
    const fetchFn = async (_url: string, init: { body: string }) => {
      bodies.push(init.body);
      return { json: async () => ({ assets: { items: [], nextPage: '2' } }) };
    };
    assert.deepEqual(await searchImmichAssets(config, {}, fetchFn), []);
    assert.equal(bodies.length, 1);
  });
});
