import { storage } from '../services/storage';
import { imageStorage, MAX_ORIGINAL_BYTES } from '../services/image-storage';
import { configService } from '../services/config';
import type { WsManager } from '../services/ws-manager';

const CONCURRENCY = 4;

let wsManager: WsManager | null = null;
export function setBackfillWsManager(wm: WsManager) { wsManager = wm; }

let backfillRunning = false;

export async function runBackfill(): Promise<{ processed: number; failed: number; skipped: number }> {
  if (backfillRunning) {
    console.log('[BACKFILL] Already running, skipping');
    return { processed: 0, failed: 0, skipped: 0 };
  }
  backfillRunning = true;

  const storagePath = imageStorage.getStoragePath();

  let processed = 0;
  let failed = 0;
  let skipped = 0;
  const skipSet = new Set<number>();

  try {
    const allImages = await storage.getAstroImages();
    const pending = allImages.filter(img =>
      !img.originalPath || !img.originalPath.startsWith(`${storagePath}/processed/`)
    );

    if (pending.length === 0) {
      console.log('[BACKFILL] Nothing to backfill');
      return { processed: 0, failed: 0, skipped: 0 };
    }

    console.log(`[BACKFILL] Starting backfill for ${pending.length} images`);
    wsManager?.broadcast('backfill-progress', { total: pending.length, processed: 0, failed: 0 });

    const config = await configService.getImmichConfig();
    if (!config.host || !config.apiKey) {
      throw new Error('Immich not configured — cannot backfill');
    }

    // Process in batches of CONCURRENCY
    for (let i = 0; i < pending.length; i += CONCURRENCY) {
      const batch = pending.slice(i, i + CONCURRENCY);
      await Promise.all(batch.map(async (img) => {
        if (skipSet.has(img.id)) { skipped++; return; }
        if (img.sourceType !== 'immich' || !img.sourceId) { skipped++; return; }

        try {
          const url = `${config.host}/api/assets/${encodeURIComponent(img.sourceId)}/original`;
          const response = await fetch(url, { headers: { 'X-API-Key': config.apiKey! } });

          if (!response.ok) {
            throw new Error(`Immich returned ${response.status}`);
          }

          const ab = await response.arrayBuffer();
          if (ab.byteLength > MAX_ORIGINAL_BYTES) {
            console.warn(`[BACKFILL] Image ${img.id} (${img.sourceId}) exceeds 500 MB — skipping`);
            skipSet.add(img.id);
            skipped++;
            return;
          }

          const bytes = Buffer.from(ab);
          const ext = img.filename?.includes('.')
            ? (img.filename.split('.').pop()?.toLowerCase() ?? 'jpg')
            : 'jpg';

          const result = await imageStorage.writeImage(img.id, bytes, ext);
          await storage.updateAstroImage(img.id, { originalPath: result.originalPath });
          processed++;
          console.log(`[BACKFILL] Stored image ${img.id}`);
        } catch (err: unknown) {
          const msg = (err as Error).message;
          console.error(`[BACKFILL] Failed for image ${img.id}: ${msg}`);
          failed++;
        }
      }));

      wsManager?.broadcast('backfill-progress', {
        total: pending.length,
        processed,
        failed,
        skipped,
      });
    }

    const summary = `Backfill complete: ${processed} stored, ${failed} failed, ${skipped} skipped`;
    console.log(`[BACKFILL] ${summary}`);
    wsManager?.broadcast('backfill-complete', { processed, failed, skipped, message: summary });

    return { processed, failed, skipped };
  } finally {
    backfillRunning = false;
  }
}

export function isBackfillRunning(): boolean {
  return backfillRunning;
}
