import crypto from 'node:crypto';
import path from 'node:path';
import type { ImageSourcePlugin, IngestResult, SourceMetadata, ConnectionResult, SourceStatus } from '@shared/types';
import { storage } from '../storage.js';
import { imageStorage } from '../image-storage.js';

type StorageService = Pick<typeof storage, 'getImageBySource' | 'createAstroImage' | 'updateAstroImage' | 'deleteAstroImage' | 'getAstroImages'>;
type FetchFn = (url: string) => Promise<{
  ok: boolean;
  status?: number;
  statusText?: string;
  arrayBuffer(): Promise<ArrayBuffer>;
  headers: { get(name: string): string | null };
}>;

export class UrlUploadSource implements ImageSourcePlugin {
  readonly sourceType = 'url';
  readonly displayName = 'Web / URL Upload';

  constructor(
    private readonly db: StorageService = storage,
    private readonly fetchFn: FetchFn = fetch,
    private readonly imgStorage: Pick<typeof imageStorage, 'writeImage'> = imageStorage,
  ) {}

  async ingest(url: string, metadata?: Partial<SourceMetadata>): Promise<IngestResult> {
    const parsed = new URL(url);

    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
      throw new Error('Only http and https URLs are supported');
    }

    const sourceId = crypto.createHash('sha256').update(url).digest('hex').slice(0, 16);

    const existing = await this.db.getImageBySource('url', sourceId);
    if (existing) {
      return { imageId: existing.id, filename: existing.filename, sourceType: 'url', sourceId };
    }

    const response = await this.fetchFn(url);
    if (!response.ok) {
      throw new Error(`Failed to fetch image: ${response.status} ${response.statusText}`);
    }

    const bytes = Buffer.from(await response.arrayBuffer());

    const contentType = response.headers.get('content-type') ?? '';
    const extFromContentType = contentType.split('/')[1]?.replace('jpeg', 'jpg') ?? '';
    const ext = extFromContentType || 'jpg';

    const basenameFromUrl = path.basename(parsed.pathname);
    const filename = metadata?.filename ?? (basenameFromUrl || 'image.jpg');

    const created = await this.db.createAstroImage({
      sourceType: 'url',
      sourceId,
      title: filename,
      filename,
      originalPath: null,
      captureDate: metadata?.captureDate ?? null,
      focalLength: metadata?.focalLength ?? null,
      aperture: metadata?.aperture ?? null,
      iso: metadata?.iso ?? null,
      exposureTime: metadata?.exposureTime ?? null,
      frameCount: 1,
      totalIntegration: null,
      telescope: '',
      camera: metadata?.camera ?? null,
      mount: '',
      filters: '',
      latitude: metadata?.latitude ?? null,
      longitude: metadata?.longitude ?? null,
      altitude: metadata?.altitude ?? null,
      plateSolved: false,
      tags: [],
      objectType: 'Deep Sky',
      description: metadata?.description ?? '',
    });

    try {
      const written = await this.imgStorage.writeImage(created.id, bytes, ext);
      await this.db.updateAstroImage(created.id, { originalPath: written.originalPath });
    } catch (err) {
      await this.db.deleteAstroImage(created.id).catch(() => {});
      throw err;
    }

    return { imageId: created.id, filename, sourceType: 'url', sourceId };
  }

  async testConnection(): Promise<ConnectionResult> {
    return { ok: true };
  }

  async getStatus(): Promise<SourceStatus> {
    const all = await this.db.getAstroImages();
    const imageCount = all.filter(img => img.sourceType === 'url').length;
    return { sourceType: 'url', lastSync: null, imageCount, healthy: true };
  }
}

export const urlUploadSource = new UrlUploadSource();
