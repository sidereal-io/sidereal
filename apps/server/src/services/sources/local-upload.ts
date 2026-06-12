import crypto from 'node:crypto';
import path from 'node:path';
import type { ImageSourcePlugin, IngestResult, SourceMetadata, ConnectionResult, SourceStatus } from '@shared/types';
import { storage } from '../storage.js';
import { imageStorage } from '../image-storage.js';

type StorageService = Pick<typeof storage, 'getImageBySource' | 'createAstroImage' | 'updateAstroImage' | 'getAstroImages'>;

export class LocalUploadSource implements ImageSourcePlugin {
  readonly sourceType = 'local';
  readonly displayName = 'Local Upload';

  constructor(private readonly db: StorageService = storage) {}

  async ingest(bytes: Buffer, filename: string, metadata?: Partial<SourceMetadata>): Promise<IngestResult> {
    const ext = path.extname(filename).replace('.', '') || 'jpg';
    const sourceId = crypto.createHash('sha256').update(bytes).digest('hex').slice(0, 16);

    const existing = await this.db.getImageBySource('local', sourceId);
    if (existing) {
      return { imageId: existing.id, filename: existing.filename, sourceType: 'local', sourceId };
    }

    const created = await this.db.createAstroImage({
      sourceType: 'local',
      sourceId,
      title: metadata?.filename ?? filename,
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

    const written = await imageStorage.writeImage(created.id, bytes, ext);
    await this.db.updateAstroImage(created.id, { originalPath: written.originalPath });

    return { imageId: created.id, filename, sourceType: 'local', sourceId };
  }

  async testConnection(): Promise<ConnectionResult> {
    return { ok: true };
  }

  async getStatus(): Promise<SourceStatus> {
    const all = await this.db.getAstroImages();
    const imageCount = all.filter(img => img.sourceType === 'local').length;
    return { sourceType: 'local', lastSync: null, imageCount, healthy: true };
  }
}

export const localUploadSource = new LocalUploadSource();
