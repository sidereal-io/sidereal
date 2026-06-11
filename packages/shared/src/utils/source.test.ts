import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { withSourceDefaults } from './source.js';

describe('withSourceDefaults', () => {
  it('derives immich source from immichId when source fields absent', () => {
    const out = withSourceDefaults({ immichId: 'abc' });
    assert.equal(out.sourceType, 'immich');
    assert.equal(out.sourceId, 'abc');
  });

  it('does not override explicit source fields', () => {
    const out = withSourceDefaults({ immichId: 'abc', sourceType: 'local', sourceId: 'xyz' });
    assert.equal(out.sourceType, 'local');
    assert.equal(out.sourceId, 'xyz');
  });

  it('leaves sourceId undefined when no immichId and no sourceId', () => {
    const out = withSourceDefaults({ immichId: null });
    assert.equal(out.sourceType, 'immich');
    assert.equal(out.sourceId, undefined);
  });

  it('preserves all other fields unchanged', () => {
    const out = withSourceDefaults({ immichId: 'abc', title: 'M31' });
    assert.equal((out as any).title, 'M31');
  });
});
