import { mkdir, rename, rm, readdir, access, writeFile } from 'node:fs/promises';
import path from 'node:path';
import sharp from 'sharp';

// Processing constants — cross-implementation contract for Rust migration
const PREVIEW_MAX_PX = 1920;
const THUMB_MAX_PX = 250;
const PREVIEW_QUALITY = 85;
const THUMB_QUALITY = 80;
export const MAX_ORIGINAL_BYTES = 500 * 1024 * 1024; // 500 MB

export class ImageNotFoundError extends Error {
  constructor(id: number, size: string) {
    super(`Image ${id} size=${size} not found on disk`);
    this.name = 'ImageNotFoundError';
  }
}

function getStoragePath(): string {
  return process.env.STORAGE_PATH ?? (
    process.env.NODE_ENV === 'production' ? '/app/data/images' : './data/images'
  );
}

export function shardBucket(id: number): string {
  return String(id % 1000).padStart(3, '0');
}

function imageDir(id: number): string {
  return path.join(getStoragePath(), 'processed', shardBucket(id), String(id));
}

export const imageStorage = {
  async writeImage(id: number, bytes: Buffer, ext: string): Promise<{ originalPath: string }> {
    if (bytes.byteLength > MAX_ORIGINAL_BYTES) {
      throw new Error(`Image ${id} exceeds 500 MB limit (${bytes.byteLength} bytes)`);
    }

    const dir = imageDir(id);
    const tmpDir = dir + '.tmp';

    try {
      await mkdir(tmpDir, { recursive: true });

      const safeExt = ext.replace(/[^a-z0-9]/gi, '').slice(0, 10) || 'jpg';
      const origName = `${id}_original.${safeExt}`;
      const origTmp = path.join(tmpDir, origName + '.part');
      const origFinal = path.join(tmpDir, origName);

      // Write original atomically
      await writeFile(origTmp, bytes);
      await rename(origTmp, origFinal);

      // Generate preview and thumbnail from same buffer
      const pipeline = sharp(bytes).rotate(); // bake EXIF orientation

      const prevTmp = path.join(tmpDir, `${id}_preview.jpg.part`);
      const prevFinal = path.join(tmpDir, `${id}_preview.jpg`);
      await pipeline
        .clone()
        .resize({ width: PREVIEW_MAX_PX, height: PREVIEW_MAX_PX, fit: 'inside', withoutEnlargement: true })
        .jpeg({ quality: PREVIEW_QUALITY, mozjpeg: true })
        .toFile(prevTmp);
      await rename(prevTmp, prevFinal);

      const thumbTmp = path.join(tmpDir, `${id}_thumb.jpg.part`);
      const thumbFinal = path.join(tmpDir, `${id}_thumb.jpg`);
      await pipeline
        .clone()
        .resize({ width: THUMB_MAX_PX, height: THUMB_MAX_PX, fit: 'inside', withoutEnlargement: true })
        .jpeg({ quality: THUMB_QUALITY, mozjpeg: true })
        .toFile(thumbTmp);
      await rename(thumbTmp, thumbFinal);

      // Brief window between rm and rename where readPath for this id throws ImageNotFoundError.
      // Safe under sequential use; concurrent same-id writes are prevented at the caller level.
      await rm(dir, { recursive: true, force: true });
      await rename(tmpDir, dir);

      return { originalPath: path.join(dir, origName) };
    } catch (err) {
      await rm(tmpDir, { recursive: true, force: true });
      throw err;
    }
  },

  async readPath(id: number, size: 'original' | 'preview' | 'thumbnail'): Promise<string> {
    const dir = imageDir(id);

    if (size === 'thumbnail') {
      const p = path.join(dir, `${id}_thumb.jpg`);
      await access(p).catch(() => { throw new ImageNotFoundError(id, size); });
      return p;
    }
    if (size === 'preview') {
      const p = path.join(dir, `${id}_preview.jpg`);
      await access(p).catch(() => { throw new ImageNotFoundError(id, size); });
      return p;
    }
    // original — find by prefix scan since extension varies
    let entries: string[];
    try {
      entries = await readdir(dir);
    } catch {
      throw new ImageNotFoundError(id, size);
    }
    const file = entries.find(e => e.startsWith(`${id}_original.`));
    if (!file) throw new ImageNotFoundError(id, size);
    return path.join(dir, file);
  },

  async deleteImage(id: number): Promise<void> {
    await rm(imageDir(id), { recursive: true, force: true });
  },

  async imageExists(id: number, size: 'original' | 'preview' | 'thumbnail'): Promise<boolean> {
    try {
      await this.readPath(id, size);
      return true;
    } catch (e) {
      if (e instanceof ImageNotFoundError) return false;
      throw e;
    }
  },

  // Exposed for orphan sweep and backfill
  getStoragePath,
  imageDir,
};
