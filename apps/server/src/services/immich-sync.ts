import { configService } from './config';
import { storage } from './storage';
import { filterRelevantTags } from './tags-utils';
import { imageStorage, MAX_ORIGINAL_BYTES } from './image-storage';
import type { AstroImage } from '@shared/types';

class ImmichSyncService {
  private filterTagsForSync(tags: string[]): string[] {
    return filterRelevantTags(tags);
  }

  private async buildMetadataTags(image: AstroImage): Promise<string[]> {
    const tags: string[] = [];

    if (image.objectType) tags.push(image.objectType);
    if (image.constellation) tags.push(image.constellation);

    try {
      const equipment = await storage.getEquipmentForImage(image.id);
      for (const eq of equipment) {
        tags.push(eq.name);
      }
    } catch { /* ignore */ }

    return tags;
  }

  private async buildMetadataItems(image: AstroImage): Promise<Array<{ key: string; value: object }>> {
    const items: Array<{ key: string; value: object }> = [];

    if (image.objectType) {
      items.push({ key: 'objectType', value: { value: image.objectType } });
    }
    if (image.constellation) {
      items.push({ key: 'constellation', value: { value: image.constellation } });
    }

    if (image.ra !== null && image.ra !== undefined) {
      items.push({ key: 'ra', value: { value: String(image.ra) } });
    }
    if (image.dec !== null && image.dec !== undefined) {
      items.push({ key: 'dec', value: { value: String(image.dec) } });
    }
    if (image.pixelScale) {
      items.push({ key: 'pixelScale', value: { value: String(image.pixelScale) } });
    }
    if (image.fieldOfView) {
      items.push({ key: 'fieldOfView', value: { value: image.fieldOfView } });
    }
    if (image.rotation) {
      items.push({ key: 'rotation', value: { value: String(image.rotation) } });
    }

    if (image.telescope) {
      items.push({ key: 'telescope', value: { value: image.telescope } });
    }
    if (image.camera) {
      items.push({ key: 'camera', value: { value: image.camera } });
    }
    if (image.mount) {
      items.push({ key: 'mount', value: { value: image.mount } });
    }
    if (image.focalLength) {
      items.push({ key: 'focalLength', value: { value: String(image.focalLength) } });
    }
    if (image.aperture) {
      items.push({ key: 'aperture', value: { value: image.aperture } });
    }
    if (image.iso) {
      items.push({ key: 'iso', value: { value: String(image.iso) } });
    }
    if (image.exposureTime) {
      items.push({ key: 'exposureTime', value: { value: image.exposureTime } });
    }
    if (image.filters) {
      items.push({ key: 'filters', value: { value: image.filters } });
    }
    if (image.frameCount) {
      items.push({ key: 'frameCount', value: { value: String(image.frameCount) } });
    }
    if (image.totalIntegration) {
      items.push({ key: 'totalIntegration', value: { value: `${image.totalIntegration}h` } });
    }

    try {
      const equipment = await storage.getEquipmentForImage(image.id);
      if (equipment.length > 0) {
        const eqSummary = equipment.map(e => `${e.name} (${e.type})`).join(', ');
        items.push({ key: 'equipment', value: { value: eqSummary } });
      }
    } catch { /* ignore */ }

    try {
      const acquisitions = await storage.getImageAcquisitions(image.id);
      if (acquisitions.length > 0) {
        const summary = acquisitions
          .map(a => `${a.filterName || 'No filter'}: ${a.frameCount}x${a.exposureTime}s`)
          .join(', ');
        const totalFrames = acquisitions.reduce((sum, a) => sum + a.frameCount, 0);
        const totalSeconds = acquisitions.reduce((sum, a) => sum + (a.frameCount * a.exposureTime), 0);
        const hours = (totalSeconds / 3600).toFixed(1);
        items.push({ key: 'acquisition', value: { value: `${summary} (${totalFrames} frames, ${hours}h)` } });
      }
    } catch { /* ignore */ }

    return items;
  }

  async syncImageMetadata(imageId: number): Promise<{ success: boolean; error?: string }> {
    const config = await configService.getImmichConfig();
    if (!config.host || !config.apiKey) {
      return { success: false, error: 'Immich is not configured. Please set the host URL and API key in Admin Settings.' };
    }
    if (!config.metadataSyncEnabled) {
      return { success: false, error: 'Metadata sync is disabled. Enable it in Admin Settings under "Metadata Sync".' };
    }

    const image = await storage.getAstroImage(imageId);
    if (!image || !image.immichId) {
      return { success: false, error: 'Image not found or not linked to Immich. Try syncing from Immich first.' };
    }

    try {
      const headers: Record<string, string> = { 'X-API-Key': config.apiKey, 'Content-Type': 'application/json' };

      // 1. Update native asset fields via PUT /api/assets/{id}
      const assetPayload: Record<string, any> = {};
      if (config.syncDescription && image.description) {
        assetPayload.description = image.description;
      }
      if (config.syncCoordinates) {
        if (image.latitude !== null && image.latitude !== undefined) {
          assetPayload.latitude = image.latitude;
        }
        if (image.longitude !== null && image.longitude !== undefined) {
          assetPayload.longitude = image.longitude;
        }
      }

      if (Object.keys(assetPayload).length > 0) {
        await fetch(`${config.host}/api/assets/${image.immichId}`, {
          method: 'PUT',
          headers,
          body: JSON.stringify(assetPayload),
        });
        console.log(`Updated asset fields for image ${imageId}: ${Object.keys(assetPayload).join(', ')}`);
      }

      // 2. Write custom astro metadata via PUT /api/assets/{id}/metadata
      const metadataItems = await this.buildMetadataItems(image);
      if (metadataItems.length > 0) {
        await fetch(`${config.host}/api/assets/${image.immichId}/metadata`, {
          method: 'PUT',
          headers,
          body: JSON.stringify({ items: metadataItems }),
        });
        console.log(`Wrote ${metadataItems.length} metadata items for image ${imageId}`);
      }

      // 3. Sync tags if enabled
      if (config.syncTags) {
        const userTags = this.filterTagsForSync(image.tags || []);
        const metadataTags = await this.buildMetadataTags(image);
        const allTags = [...new Set([...userTags, ...metadataTags])];
        if (allTags.length > 0) {
          await this.syncTags(config.host, config.apiKey, image.immichId, allTags);
        }
      }

      console.log(`Synced metadata to Immich for image ${imageId} (asset ${image.immichId})`);
      return { success: true };
    } catch (error: any) {
      const msg = error.message;
      console.error(`Failed to sync metadata for image ${imageId}:`, msg);
      return { success: false, error: typeof msg === 'string' ? msg : JSON.stringify(msg) };
    }
  }

  private async syncTags(host: string, apiKey: string, immichAssetId: string, tags: string[]): Promise<void> {
    const headers: Record<string, string> = { 'X-API-Key': apiKey, 'Content-Type': 'application/json' };

    let existingTags: Array<{ id: string; name: string }> = [];
    try {
      const listRes = await fetch(`${host}/api/tags`, { headers: { 'X-API-Key': apiKey } });
      existingTags = await listRes.json() as Array<{ id: string; name: string }> || [];
    } catch {
      console.warn('Failed to fetch existing Immich tags');
    }

    for (const tagName of tags) {
      try {
        let tagId: string | undefined;

        const existing = existingTags.find((t) => t.name === tagName);
        if (existing) {
          tagId = existing.id;
        } else {
          const createRes = await fetch(`${host}/api/tags`, {
            method: 'POST',
            headers,
            body: JSON.stringify({ name: tagName }),
          });
          const createData = await createRes.json() as Record<string, unknown>;
          tagId = createData?.id as string | undefined;
          if (tagId) {
            existingTags.push({ id: tagId, name: tagName });
          }
        }

        if (tagId) {
          try {
            await fetch(`${host}/api/tags/${tagId}/assets`, {
              method: 'PUT',
              headers,
              body: JSON.stringify({ ids: [immichAssetId] }),
            });
          } catch (e: any) {
            console.warn(`Failed to assign tag "${tagName}":`, e.message);
          }
        }
      } catch (err: any) {
        console.warn(`Failed to sync tag "${tagName}":`, err.message);
      }
    }
  }

  private async getAssetOriginal(
    immichId: string,
    config: { host: string; apiKey: string }
  ): Promise<{ bytes: Buffer; filename: string }> {
    const url = `${config.host}/api/assets/${encodeURIComponent(immichId)}/original`;
    const response = await fetch(url, {
      headers: { 'X-API-Key': config.apiKey },
    });

    if (!response.ok) {
      throw new Error(`Immich returned ${response.status} for original of asset ${immichId}`);
    }

    const contentLength = response.headers.get('content-length');
    if (contentLength && parseInt(contentLength, 10) > MAX_ORIGINAL_BYTES) {
      throw new Error(`Original for asset ${immichId} is ${contentLength} bytes — exceeds 500 MB limit, skipping`);
    }

    const arrayBuffer = await response.arrayBuffer();
    if (arrayBuffer.byteLength > MAX_ORIGINAL_BYTES) {
      throw new Error(`Original for asset ${immichId} is ${arrayBuffer.byteLength} bytes — exceeds 500 MB limit, skipping`);
    }

    const disposition = response.headers.get('content-disposition') ?? '';
    const filenameMatch = disposition.match(/filename="?([^";]+)"?/i);
    const filename = filenameMatch?.[1] ?? `${immichId}.jpg`;

    return { bytes: Buffer.from(arrayBuffer), filename };
  }

  /**
   * Sync images from Immich (import new images, remove deleted ones).
   * Extracted from the /sync-immich route so cron can call it directly.
   */
  async syncImagesFromImmich(): Promise<{ syncedCount: number; removedCount: number; message: string }> {
    const config = await configService.getImmichConfig();

    if (!config.host || !config.apiKey) {
      throw new Error('Immich configuration missing. Please configure in admin settings or set environment variables.');
    }

    let allAssets: Record<string, unknown>[] = [];

    if (config.syncByAlbum) {
      if (!Array.isArray(config.selectedAlbumIds) || config.selectedAlbumIds.length === 0) {
        throw new Error('Sync by album is enabled, but no albums are selected.');
      }

      const albumsRes = await fetch(`${config.host}/api/albums`, {
        headers: { 'X-API-Key': config.apiKey },
      });
      const albumsData = await albumsRes.json() as Record<string, unknown>[];
      const albumsToSync = albumsData.filter((a) => config.selectedAlbumIds.includes(a.id as string));

      for (const album of albumsToSync) {
        if (album.id && (album.assetCount as number) > 0) {
          try {
            console.log(`Fetching assets from album: ${album.albumName} (${album.assetCount} assets)`);
            const albumRes = await fetch(`${config.host}/api/albums/${album.id}`, {
              headers: { 'X-API-Key': config.apiKey },
            });
            const albumData = await albumRes.json() as Record<string, unknown>;
            if (albumData && albumData.assets && Array.isArray(albumData.assets)) {
              allAssets.push(...(albumData.assets as Record<string, unknown>[]));
              console.log(`Added ${(albumData.assets as unknown[]).length} assets from album ${album.albumName}`);
            }
          } catch (albumError: unknown) {
            const err = albumError as Error;
            console.warn(
              `Failed to get assets from album ${album.albumName} (${album.id}):`,
              err.message,
            );
          }
        }
      }
    } else {
      // Sync all assets from library using metadata search with pagination
      console.log('Fetching all assets from Immich library via metadata search...');
      let page = 1;
      const pageSize = 1000;
      let hasMore = true;

      while (hasMore) {
        const response = await fetch(`${config.host}/api/search/metadata`, {
          method: 'POST',
          headers: { 'X-API-Key': config.apiKey, 'Content-Type': 'application/json' },
          body: JSON.stringify({ size: pageSize, page, type: 'IMAGE' }),
        });
        const data = await response.json() as Record<string, unknown>;

        const assets = data?.assets as Record<string, unknown> | undefined;
        const items = (assets?.items || []) as Record<string, unknown>[];
        allAssets.push(...items);
        console.log(`Page ${page}: fetched ${items.length} assets (${allAssets.length} total)`);

        const nextPage = assets?.nextPage;
        if (nextPage != null && items.length > 0) {
          page = nextPage as number;
        } else {
          hasMore = false;
        }
      }

      console.log(`Fetched ${allAssets.length} total assets from Immich library`);
    }

    // De-duplicate assets by ID (important when assets are in multiple albums)
    const uniqueAssetsMap = new Map();
    for (const asset of allAssets) {
      uniqueAssetsMap.set(asset.id, asset);
    }
    allAssets = Array.from(uniqueAssetsMap.values());

    console.log(`Found ${allAssets.length} total unique assets in Immich`);

    // Remove images from our app that no longer exist in Immich
    const immichAssetIds = new Set(allAssets.map((asset) => asset.id));
    const allAppImages = await storage.getAstroImages();
    let removedCount = 0;
    for (const img of allAppImages) {
      if (img.immichId && !immichAssetIds.has(img.immichId)) {
        await storage.deleteAstroImage(img.id);
        removedCount++;
        console.log(`Removed image ${img.title || img.id} (immichId: ${img.immichId}) because it no longer exists in Immich`);
      }
    }
    if (removedCount > 0) {
      console.log(`Removed ${removedCount} images that no longer exist in Immich.`);
    }

    let syncedCount = 0;

    for (const asset of allAssets) {
      // Check if image already exists
      const existing = await storage.getAstroImageByImmichId(asset.id as string);
      if (existing) {
        continue;
      }

      const exifInfo = asset.exifInfo as Record<string, unknown> | undefined;
      const originalFileName = String(asset.originalFileName || '');
      const ext = originalFileName.includes('.')
        ? (originalFileName.split('.').pop()?.toLowerCase() ?? 'jpg')
        : 'jpg';

      let created: { id: number } | null = null;
      try {
        const { bytes } = await this.getAssetOriginal(String(asset.id), config);

        const placeholder = {
          immichId: String(asset.id),
          title: originalFileName || String(asset.id),
          filename: originalFileName,
          originalPath: null,
          captureDate: asset.fileCreatedAt ? new Date(asset.fileCreatedAt as string) : null,
          focalLength: exifInfo?.focalLength ? Number(exifInfo.focalLength) : null,
          aperture: exifInfo?.fNumber ? `f/${exifInfo.fNumber}` : null,
          iso: exifInfo?.iso ? Number(exifInfo.iso) : null,
          exposureTime: exifInfo?.exposureTime ? String(exifInfo.exposureTime) : null,
          frameCount: 1,
          totalIntegration: exifInfo?.exposureTime ? parseFloat(String(exifInfo.exposureTime)) / 3600 : null,
          telescope: exifInfo?.lensModel ? String(exifInfo.lensModel) : '',
          camera: exifInfo?.make && exifInfo?.model ? `${exifInfo.make} ${exifInfo.model}` : null,
          mount: '',
          filters: '',
          latitude: exifInfo?.latitude ? Number(exifInfo.latitude) : null,
          longitude: exifInfo?.longitude ? Number(exifInfo.longitude) : null,
          altitude: exifInfo?.altitude ? Number(exifInfo.altitude) : null,
          plateSolved: false,
          tags: ['astrophotography'],
          objectType: 'Deep Sky',
          description: String(exifInfo?.description || ''),
        };

        created = await storage.createAstroImage(placeholder);
        const result = await imageStorage.writeImage(created.id, bytes, ext);
        await storage.updateAstroImage(created.id, { originalPath: result.originalPath });
        syncedCount++;
        console.log(`Synced and stored asset: ${originalFileName} (id=${created.id})`);
      } catch (err: unknown) {
        const msg = (err as Error).message;
        console.warn(`Skipping asset ${String(asset.id)} due to storage error: ${msg}`);
        if (created) {
          await storage.deleteAstroImage(created.id).catch(() => {});
        }
      }
    }

    const message = `Successfully synced ${syncedCount} new images from Immich. Removed ${removedCount} images no longer in Immich.`;
    return { syncedCount, removedCount, message };
  }

  async syncAllImages(): Promise<{ synced: number; failed: number; errors: string[] }> {
    const images = await storage.getAstroImages();
    let synced = 0;
    let failed = 0;
    const errors: string[] = [];

    for (const image of images) {
      if (!image.immichId) continue;

      const result = await this.syncImageMetadata(image.id);
      if (result.success) {
        synced++;
      } else {
        failed++;
        if (result.error) errors.push(`Image ${image.id}: ${result.error}`);
      }
    }

    return { synced, failed, errors };
  }
}

export const immichSyncService = new ImmichSyncService();
