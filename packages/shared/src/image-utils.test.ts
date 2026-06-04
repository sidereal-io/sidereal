import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { computeImageSummary, groupAndEnrichTargets } from './image-utils';
import type { AcquisitionInput, EquipmentInput, ImageInput, CatalogInput } from './image-utils';

// ---------------------------------------------------------------------------
// computeImageSummary
// ---------------------------------------------------------------------------
describe('computeImageSummary', () => {
  it('returns empty object for no acquisitions and no equipment', () => {
    const result = computeImageSummary([], []);
    assert.deepEqual(result, {});
  });

  it('computes total frames from acquisitions', () => {
    const acqs: AcquisitionInput[] = [
      { frameCount: 10, exposureTime: 300, filterName: 'Ha' },
      { frameCount: 20, exposureTime: 300, filterName: 'OIII' },
    ];
    const result = computeImageSummary(acqs, []);
    assert.equal(result.frameCount, 30);
  });

  it('computes total integration in hours', () => {
    const acqs: AcquisitionInput[] = [
      { frameCount: 12, exposureTime: 300, filterName: null }, // 12 * 300 = 3600s = 1h
    ];
    const result = computeImageSummary(acqs, []);
    assert.equal(result.totalIntegration, 1);
  });

  it('rounds integration to 3 decimal places', () => {
    const acqs: AcquisitionInput[] = [
      { frameCount: 7, exposureTime: 180, filterName: null }, // 7*180 = 1260s = 0.35h
    ];
    const result = computeImageSummary(acqs, []);
    assert.equal(result.totalIntegration, 0.35);
  });

  it('computes filter list from unique filter names', () => {
    const acqs: AcquisitionInput[] = [
      { frameCount: 10, exposureTime: 300, filterName: 'Ha' },
      { frameCount: 10, exposureTime: 300, filterName: 'OIII' },
      { frameCount: 10, exposureTime: 300, filterName: 'Ha' }, // duplicate
    ];
    const result = computeImageSummary(acqs, []);
    assert.equal(result.filters, 'Ha, OIII');
  });

  it('returns null filters when no filter names present', () => {
    const acqs: AcquisitionInput[] = [
      { frameCount: 10, exposureTime: 300, filterName: null },
    ];
    const result = computeImageSummary(acqs, []);
    assert.equal(result.filters, null);
  });

  it('formats single exposure time', () => {
    const acqs: AcquisitionInput[] = [
      { frameCount: 10, exposureTime: 300, filterName: null },
      { frameCount: 5, exposureTime: 300, filterName: null },
    ];
    const result = computeImageSummary(acqs, []);
    assert.equal(result.exposureTime, '300s');
  });

  it('formats exposure time range for multiple distinct values', () => {
    const acqs: AcquisitionInput[] = [
      { frameCount: 10, exposureTime: 120, filterName: null },
      { frameCount: 5, exposureTime: 300, filterName: null },
    ];
    const result = computeImageSummary(acqs, []);
    assert.equal(result.exposureTime, '120s - 300s');
  });

  it('extracts telescope focal length from equipment', () => {
    const equip: EquipmentInput[] = [
      { type: 'telescope', specifications: { focalLength: 800, focalRatio: 'f/4' } },
      { type: 'camera', specifications: { sensorType: 'CMOS' } },
    ];
    const result = computeImageSummary([], equip);
    assert.equal(result.focalLength, 800);
    assert.equal(result.aperture, 'f/4');
  });

  it('ignores non-telescope equipment', () => {
    const equip: EquipmentInput[] = [
      { type: 'camera', specifications: { focalLength: 999 } },
    ];
    const result = computeImageSummary([], equip);
    assert.equal(result.focalLength, undefined);
  });

  it('handles telescope with null specifications', () => {
    const equip: EquipmentInput[] = [
      { type: 'telescope', specifications: null },
    ];
    const result = computeImageSummary([], equip);
    assert.equal(result.focalLength, undefined);
  });

  it('combines acquisitions and equipment', () => {
    const acqs: AcquisitionInput[] = [
      { frameCount: 20, exposureTime: 600, filterName: 'L' },
    ];
    const equip: EquipmentInput[] = [
      { type: 'telescope', specifications: { focalLength: 530 } },
    ];
    const result = computeImageSummary(acqs, equip);
    assert.equal(result.frameCount, 20);
    assert.ok(result.totalIntegration! > 3.3);
    assert.equal(result.focalLength, 530);
    assert.equal(result.filters, 'L');
  });
});

// ---------------------------------------------------------------------------
// groupAndEnrichTargets
// ---------------------------------------------------------------------------
describe('groupAndEnrichTargets', () => {
  function makeImage(overrides: Partial<ImageInput> = {}): ImageInput {
    return {
      id: 1,
      title: 'Test Image',
      targetName: null,
      objectType: null,
      constellation: null,
      captureDate: null,
      totalIntegration: null,
      ...overrides,
    };
  }

  function makeCatalog(overrides: Partial<CatalogInput> = {}): CatalogInput {
    return {
      name: 'NGC 224',
      messier: 'M 31',
      type: 'G',
      constellation: 'And',
      vMag: 3.44,
      commonNames: 'Andromeda Galaxy',
      ...overrides,
    };
  }

  it('returns empty array for no images', () => {
    const result = groupAndEnrichTargets([], []);
    assert.deepEqual(result, []);
  });

  it('groups images by targetName', () => {
    const images = [
      makeImage({ id: 1, targetName: 'NGC 224', title: 'Andromeda 1' }),
      makeImage({ id: 2, targetName: 'NGC 224', title: 'Andromeda 2' }),
      makeImage({ id: 3, targetName: 'NGC 7000', title: 'North America' }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result.length, 2);
    const ngc224 = result.find(t => t.targetName === 'NGC 224');
    assert.ok(ngc224);
    assert.equal(ngc224.imageCount, 2);
    assert.deepEqual(ngc224.imageIds, [1, 2]);
  });

  it('falls back to title when targetName is null', () => {
    const images = [
      makeImage({ id: 1, targetName: null, title: 'Orion' }),
      makeImage({ id: 2, targetName: null, title: 'Orion' }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result.length, 1);
    assert.equal(result[0].targetName, 'Orion');
    assert.equal(result[0].imageCount, 2);
  });

  it('enriches with catalog data when target matches catalog name', () => {
    const images = [makeImage({ id: 1, targetName: 'NGC 224' })];
    const catalog = [makeCatalog()];
    const result = groupAndEnrichTargets(images, catalog);
    assert.equal(result[0].objectType, 'G');
    assert.equal(result[0].constellation, 'And');
    assert.equal(result[0].vMag, 3.44);
    assert.equal(result[0].commonNames, 'Andromeda Galaxy');
  });

  it('enriches via Messier designation', () => {
    const images = [makeImage({ id: 1, targetName: 'M 31' })];
    const catalog = [makeCatalog({ name: 'NGC 224', messier: 'M 31' })];
    const result = groupAndEnrichTargets(images, catalog);
    assert.equal(result[0].objectType, 'G');
    assert.equal(result[0].commonNames, 'Andromeda Galaxy');
  });

  it('uses image metadata when catalog has no match', () => {
    const images = [makeImage({ id: 1, targetName: 'Unknown', objectType: 'Neb', constellation: 'Ori' })];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result[0].objectType, 'Neb');
    assert.equal(result[0].constellation, 'Ori');
    assert.equal(result[0].vMag, null);
  });

  it('computes total integration hours across group', () => {
    const images = [
      makeImage({ id: 1, targetName: 'NGC 224', totalIntegration: 2.5 }),
      makeImage({ id: 2, targetName: 'NGC 224', totalIntegration: 1.5 }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result[0].totalIntegrationHours, 4);
  });

  it('rounds integration to 3 decimal places', () => {
    const images = [
      makeImage({ id: 1, targetName: 'X', totalIntegration: 0.1111 }),
      makeImage({ id: 2, targetName: 'X', totalIntegration: 0.2222 }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result[0].totalIntegrationHours, 0.333);
  });

  it('picks thumbnail from most recent image', () => {
    const images = [
      makeImage({ id: 1, targetName: 'X', captureDate: '2024-01-01' }),
      makeImage({ id: 2, targetName: 'X', captureDate: '2024-06-01' }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result[0].thumbnailImageId, 2);
  });

  it('returns latest capture date as ISO string', () => {
    const images = [
      makeImage({ id: 1, targetName: 'X', captureDate: '2024-01-15T00:00:00Z' }),
      makeImage({ id: 2, targetName: 'X', captureDate: '2024-06-20T00:00:00Z' }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.ok(result[0].latestCaptureDate!.includes('2024-06-20'));
  });

  it('handles null capture dates without crashing', () => {
    const images = [
      makeImage({ id: 1, targetName: 'X', captureDate: null }),
      makeImage({ id: 2, targetName: 'X', captureDate: null }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result[0].latestCaptureDate, null);
  });

  it('sorts by image count descending', () => {
    const images = [
      makeImage({ id: 1, targetName: 'A' }),
      makeImage({ id: 2, targetName: 'B' }),
      makeImage({ id: 3, targetName: 'B' }),
      makeImage({ id: 4, targetName: 'B' }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result[0].targetName, 'B');
    assert.equal(result[0].imageCount, 3);
    assert.equal(result[1].targetName, 'A');
  });

  it('breaks ties by latest capture date', () => {
    const images = [
      makeImage({ id: 1, targetName: 'A', captureDate: '2024-01-01' }),
      makeImage({ id: 2, targetName: 'B', captureDate: '2024-06-01' }),
    ];
    const result = groupAndEnrichTargets(images, []);
    // Both have 1 image, B has later date → B first
    assert.equal(result[0].targetName, 'B');
    assert.equal(result[1].targetName, 'A');
  });

  it('handles null totalIntegration as 0', () => {
    const images = [
      makeImage({ id: 1, targetName: 'X', totalIntegration: null }),
      makeImage({ id: 2, targetName: 'X', totalIntegration: 2.0 }),
    ];
    const result = groupAndEnrichTargets(images, []);
    assert.equal(result[0].totalIntegrationHours, 2);
  });
});
