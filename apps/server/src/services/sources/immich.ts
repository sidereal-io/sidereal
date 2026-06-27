import type { ImageSourcePlugin, ConnectionResult, SourceStatus } from '@shared/types';
import { storage } from '../storage.js';
import { imageStorage, MAX_ORIGINAL_BYTES } from '../image-storage.js';
import { configService } from '../config.js';

type StorageService = Pick<typeof storage, 'getImageBySource' | 'createAstroImage' | 'updateAstroImage' | 'deleteAstroImage' | 'getAstroImages'>;
type ConfigService = Pick<typeof configService, 'getImmichConfig'>;
type FetchResponse = {
  ok: boolean;
  status: number;
  json(): Promise<unknown>;
  arrayBuffer(): Promise<ArrayBuffer>;
  headers: { get(name: string): string | null };
};
type FetchFn = (url: string, init?: { method?: string; headers?: Record<string, string>; body?: string }) => Promise<FetchResponse>;

export class ImmichImageSource implements ImageSourcePlugin {
  readonly sourceType = 'immich';
  readonly displayName = 'Immich';

  constructor(
    private readonly db: StorageService = storage,
    private readonly imgStorage: Pick<typeof imageStorage, 'writeImage'> = imageStorage,
    private readonly cfg: ConfigService = configService,
    private readonly fetchFn: FetchFn = fetch,
  ) {}

  async testConnection(): Promise<ConnectionResult> {
    const config = await this.cfg.getImmichConfig();
    if (!config.host || !config.apiKey) {
      return { ok: false, message: 'Immich host and API key are not configured' };
    }
    try {
      const res = await this.fetchFn(`${config.host}/api/albums?take=1`, {
        headers: { 'X-API-Key': config.apiKey, Accept: 'application/json' },
      });
      if (!res.ok) return { ok: false, message: `Immich returned status ${res.status}` };
      return { ok: true };
    } catch (err) {
      return { ok: false, message: (err as Error).message };
    }
  }

  async getStatus(): Promise<SourceStatus> {
    const all = await this.db.getAstroImages();
    const imageCount = all.filter(img => img.sourceType === 'immich').length;
    return { sourceType: 'immich', lastSync: null, imageCount, healthy: true };
  }

  private async getAssetOriginal(immichId: string, config: { host: string; apiKey: string }): Promise<{ bytes: Buffer; filename: string }> {
    const url = `${config.host}/api/assets/${encodeURIComponent(immichId)}/original`;
    const response = await this.fetchFn(url, { headers: { 'X-API-Key': config.apiKey } });
    if (!response.ok) throw new Error(`Immich returned ${response.status} for original of asset ${immichId}`);
    const contentLength = response.headers.get('content-length');
    if (contentLength && parseInt(contentLength, 10) > MAX_ORIGINAL_BYTES) {
      throw new Error(`Original for asset ${immichId} is ${contentLength} bytes — exceeds limit, skipping`);
    }
    const arrayBuffer = await response.arrayBuffer();
    if (arrayBuffer.byteLength > MAX_ORIGINAL_BYTES) {
      throw new Error(`Original for asset ${immichId} is ${arrayBuffer.byteLength} bytes — exceeds limit, skipping`);
    }
    const disposition = response.headers.get('content-disposition') ?? '';
    const filenameMatch = disposition.match(/filename="?([^";]+)"?/i);
    const filename = filenameMatch?.[1] ?? `${immichId}.jpg`;
    return { bytes: Buffer.from(arrayBuffer), filename };
  }

  async sync(): Promise<{ syncedCount: number; removedCount: number; message: string }> {
    const config = await this.cfg.getImmichConfig();
    if (!config.host || !config.apiKey) {
      throw new Error('Immich configuration missing. Please configure in admin settings.');
    }

    let allAssets: Record<string, unknown>[] = [];
    if (config.syncByAlbum) {
      if (!Array.isArray(config.selectedAlbumIds) || config.selectedAlbumIds.length === 0) {
        throw new Error('Sync by album is enabled, but no albums are selected.');
      }
      const albumsRes = await this.fetchFn(`${config.host}/api/albums`, { headers: { 'X-API-Key': config.apiKey } });
      const albumsData = (await albumsRes.json()) as Record<string, unknown>[];
      const albumsToSync = albumsData.filter((a) => config.selectedAlbumIds.includes(a.id as string));
      for (const album of albumsToSync) {
        if (album.id && (album.assetCount as number) > 0) {
          try {
            const albumRes = await this.fetchFn(`${config.host}/api/albums/${album.id}`, { headers: { 'X-API-Key': config.apiKey } });
            const albumData = (await albumRes.json()) as Record<string, unknown>;
            if (albumData?.assets && Array.isArray(albumData.assets)) {
              allAssets.push(...(albumData.assets as Record<string, unknown>[]));
            }
          } catch (albumError) {
            console.warn(`Failed to get assets from album ${album.id}:`, (albumError as Error).message);
          }
        }
      }
    } else {
      let page = 1;
      const pageSize = 1000;
      let hasMore = true;
      while (hasMore) {
        const response = await this.fetchFn(`${config.host}/api/search/metadata`, {
          method: 'POST',
          headers: { 'X-API-Key': config.apiKey, 'Content-Type': 'application/json' },
          body: JSON.stringify({ size: pageSize, page, type: 'IMAGE' }),
        });
        const data = (await response.json()) as Record<string, unknown>;
        const assets = data?.assets as Record<string, unknown> | undefined;
        const items = (assets?.items || []) as Record<string, unknown>[];
        allAssets.push(...items);
        const nextPage = assets?.nextPage;
        if (nextPage != null && items.length > 0) page = nextPage as number;
        else hasMore = false;
      }
    }

    const uniqueAssetsMap = new Map<unknown, Record<string, unknown>>();
    for (const asset of allAssets) uniqueAssetsMap.set(asset.id, asset);
    allAssets = Array.from(uniqueAssetsMap.values());

    const immichAssetIds = new Set(allAssets.map((asset) => String(asset.id)));
    const allAppImages = await this.db.getAstroImages();
    let removedCount = 0;
    for (const img of allAppImages) {
      if (img.sourceType === 'immich' && img.sourceId && !immichAssetIds.has(img.sourceId)) {
        await this.db.deleteAstroImage(img.id);
        removedCount++;
      }
    }

    let syncedCount = 0;
    for (const asset of allAssets) {
      const assetId = String(asset.id);
      const existing = await this.db.getImageBySource('immich', assetId);
      if (existing) continue;

      const exifInfo = asset.exifInfo as Record<string, unknown> | undefined;
      const originalFileName = String(asset.originalFileName || '');
      const ext = originalFileName.includes('.') ? (originalFileName.split('.').pop()?.toLowerCase() ?? 'jpg') : 'jpg';

      let created: { id: number } | null = null;
      try {
        const { bytes } = await this.getAssetOriginal(assetId, config);
        created = await this.db.createAstroImage({
          sourceType: 'immich',
          sourceId: assetId,
          title: originalFileName || assetId,
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
        });
        const result = await this.imgStorage.writeImage(created.id, bytes, ext);
        await this.db.updateAstroImage(created.id, { originalPath: result.originalPath });
        syncedCount++;
      } catch (err) {
        console.warn(`Skipping asset ${assetId}: ${(err as Error).message}`);
        if (created) await this.db.deleteAstroImage(created.id).catch(() => {});
      }
    }

    const message = `Successfully synced ${syncedCount} new images from Immich. Removed ${removedCount} images no longer in Immich.`;
    return { syncedCount, removedCount, message };
  }
}

export const immichImageSource = new ImmichImageSource();
