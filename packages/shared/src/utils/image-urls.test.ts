import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { imageUrl } from './image-urls.js';

describe('imageUrl', () => {
  it('returns /api/images/:id/thumbnail for thumbnail size', () => {
    assert.equal(imageUrl(42, 'thumbnail'), '/api/images/42/thumbnail');
  });

  it('returns /api/images/:id/preview for preview size', () => {
    assert.equal(imageUrl(42, 'preview'), '/api/images/42/preview');
  });

  it('returns /api/images/:id/original for original size', () => {
    assert.equal(imageUrl(42, 'original'), '/api/images/42/original');
  });
});
