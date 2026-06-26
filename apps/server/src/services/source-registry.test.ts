import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { sourceRegistry } from './source-registry.js';

describe('SourceRegistry', () => {
  it('lists built-in sources after import', () => {
    const sources = sourceRegistry.list();
    const types = sources.map(s => s.sourceType);
    assert.ok(types.includes('local'), 'local source should be registered');
    assert.ok(types.includes('url'), 'url source should be registered');
  });

  it('get returns plugin by sourceType', () => {
    const plugin = sourceRegistry.get('local');
    assert.ok(plugin, 'should return local plugin');
    assert.equal(plugin?.sourceType, 'local');
  });

  it('get returns undefined for unknown sourceType', () => {
    assert.equal(sourceRegistry.get('unknown'), undefined);
  });

  it('registers the immich source', () => {
    const plugin = sourceRegistry.get('immich');
    assert.ok(plugin, 'immich source should be registered');
    assert.equal(plugin?.sourceType, 'immich');
  });
});
