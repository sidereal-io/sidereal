/**
 * Pure functions for computing image summaries and target grouping.
 * Extracted from storage service for testability.
 */

export interface AcquisitionInput {
  frameCount: number;
  exposureTime: number;
  filterName: string | null;
}

export interface EquipmentInput {
  type: string;
  specifications: Record<string, unknown> | null;
}

export interface ImageSummaryUpdates {
  frameCount?: number;
  totalIntegration?: number;
  filters?: string | null;
  exposureTime?: string;
  focalLength?: number;
  aperture?: string;
}

/**
 * Compute image summary fields from acquisition data + linked equipment.
 * Returns only the fields that should be updated (empty object if nothing to compute).
 */
export function computeImageSummary(
  acquisitions: AcquisitionInput[],
  linkedEquipment: EquipmentInput[],
): ImageSummaryUpdates {
  const updates: ImageSummaryUpdates = {};

  if (acquisitions.length > 0) {
    const totalFrames = acquisitions.reduce((sum, a) => sum + a.frameCount, 0);
    const totalSeconds = acquisitions.reduce((sum, a) => sum + (a.frameCount * a.exposureTime), 0);
    const filterNames = [...new Set(
      acquisitions.map(a => a.filterName).filter(Boolean) as string[]
    )];

    updates.frameCount = totalFrames;
    updates.totalIntegration = Math.round((totalSeconds / 3600) * 1000) / 1000;
    updates.filters = filterNames.join(', ') || null;

    const exposureTimes = [...new Set(acquisitions.map(a => a.exposureTime))];
    if (exposureTimes.length === 1) {
      updates.exposureTime = `${exposureTimes[0]}s`;
    } else {
      const min = Math.min(...exposureTimes);
      const max = Math.max(...exposureTimes);
      updates.exposureTime = `${min}s - ${max}s`;
    }
  }

  const telescope = linkedEquipment.find(e => e.type === 'telescope');
  if (telescope && telescope.specifications) {
    const specs = telescope.specifications;
    if (specs.focalLength) updates.focalLength = Number(specs.focalLength);
    if (specs.focalRatio) updates.aperture = String(specs.focalRatio);
  }

  return updates;
}

export interface ImageInput {
  id: number;
  title: string;
  targetName: string | null;
  objectType: string | null;
  constellation: string | null;
  captureDate: Date | string | null;
  totalIntegration: number | null;
}

export interface CatalogInput {
  name: string;
  messier: string | null;
  type: string | null;
  constellation: string | null;
  vMag: number | null;
  commonNames: string | null;
}

export interface TargetSummary {
  targetName: string;
  imageCount: number;
  totalIntegrationHours: number;
  thumbnailImageId: number | null;
  objectType: string | null;
  constellation: string | null;
  vMag: number | null;
  commonNames: string | null;
  latestCaptureDate: string | null;
  imageIds: number[];
}

/**
 * Group images by target, enrich with catalog data, and sort.
 * Pure function — no DB access.
 */
export function groupAndEnrichTargets(
  images: ImageInput[],
  catalogObjects: CatalogInput[],
): TargetSummary[] {
  // Build catalog lookup (by name and Messier designation)
  const catalogMap = new Map<string, CatalogInput>();
  for (const obj of catalogObjects) {
    catalogMap.set(obj.name, obj);
    if (obj.messier) catalogMap.set(obj.messier, obj);
  }

  // Group images by targetName (fallback to title)
  const groups = new Map<string, ImageInput[]>();
  for (const img of images) {
    const key = img.targetName || img.title;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(img);
  }

  const targets: TargetSummary[] = [];
  for (const [targetName, groupImages] of groups) {
    // Sort by capture date descending
    groupImages.sort((a, b) => {
      const aDate = a.captureDate ? new Date(a.captureDate).getTime() : 0;
      const bDate = b.captureDate ? new Date(b.captureDate).getTime() : 0;
      return bDate - aDate;
    });

    const mostRecent = groupImages[0];
    const totalHours = groupImages.reduce((sum, img) => sum + (img.totalIntegration || 0), 0);

    const catalogObj = catalogMap.get(targetName);

    targets.push({
      targetName,
      imageCount: groupImages.length,
      totalIntegrationHours: Math.round(totalHours * 1000) / 1000,
      thumbnailImageId: mostRecent?.id ?? null,
      objectType: catalogObj?.type || mostRecent.objectType || null,
      constellation: catalogObj?.constellation || mostRecent.constellation || null,
      vMag: catalogObj?.vMag || null,
      commonNames: catalogObj?.commonNames || null,
      latestCaptureDate: mostRecent.captureDate ? new Date(mostRecent.captureDate).toISOString() : null,
      imageIds: groupImages.map(img => img.id),
    });
  }

  // Sort by image count descending, then by latest capture date
  targets.sort((a, b) => b.imageCount - a.imageCount ||
    (new Date(b.latestCaptureDate || 0).getTime()) - (new Date(a.latestCaptureDate || 0).getTime()));

  return targets;
}
