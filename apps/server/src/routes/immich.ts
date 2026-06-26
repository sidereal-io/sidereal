import { Hono } from 'hono';
import { configService } from '../services/config';
import { storage } from '../services/storage';
import { handleRouteError } from './route-utils';

const app = new Hono();

// Sync images from Immich
app.post('/sync-immich', async (c) => {
  try {
    const { sourceRegistry } = await import('../services/source-registry');
    const immich = sourceRegistry.get('immich') as unknown as { sync(): Promise<{ syncedCount: number; removedCount: number; message: string }> } | undefined;
    if (!immich?.sync) {
      return c.json({ message: 'Immich source is not available' }, 503);
    }
    const result = await immich.sync();
    return c.json(result);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to sync with Immich');
  }
});

// Test Immich connection
app.post('/test-immich-connection', async (c) => {
  try {
    let { host, apiKey } = await c.req.json();

    // If a redacted key was sent, use the stored key
    if (apiKey && apiKey.startsWith('••••')) {
      const config = await configService.getImmichConfig();
      apiKey = config.apiKey;
    }

    if (!host || !apiKey) {
      return c.json({ message: 'Host and API key are required' }, 400);
    }

    // Validate URL format and protocol
    try {
      const url = new URL(host);
      if (!['http:', 'https:'].includes(url.protocol)) {
        return c.json({ message: 'Only HTTP and HTTPS protocols are allowed' }, 400);
      }
    } catch {
      return c.json({ message: 'Invalid URL format' }, 400);
    }

    // Test the connection by trying to get albums
    const response = await fetch(`${host}/api/albums?take=1`, {
      headers: {
        'X-API-Key': apiKey,
        'Accept': 'application/json',
      },
      signal: AbortSignal.timeout(10000),
    });

    // Check if response is JSON
    const contentType = response.headers.get('content-type') || '';
    if (!contentType.includes('application/json')) {
      return c.json({
        message: `Server returned non-JSON response (${contentType}). Please check the host URL.`,
      }, 500);
    }

    if (response.ok) {
      return c.json({ success: true, message: 'Connection successful!' });
    } else {
      return c.json({ success: false, message: `Connection failed with status: ${response.status}` });
    }
  } catch (error: unknown) {
    const err = error as Error & { code?: string };

    let errorMessage = 'Connection failed';
    if (err.code === 'ECONNREFUSED') {
      errorMessage = 'Cannot connect to Immich server. Please check the host URL.';
    } else if (err.code === 'ENOTFOUND') {
      errorMessage = 'Host not found. Please check the host URL.';
    } else if (err.message) {
      errorMessage = err.message;
    }

    return c.json({ message: errorMessage }, 500);
  }
});

// Fetch albums from Immich
app.post('/albums', async (c) => {
  try {
    let { host, apiKey } = await c.req.json();

    // If a redacted key was sent, use the stored key
    if (apiKey && apiKey.startsWith('••••')) {
      const config = await configService.getImmichConfig();
      apiKey = config.apiKey;
    }

    if (!host || !apiKey) {
      return c.json({ message: 'Host and API key are required' }, 400);
    }

    // Validate URL format and protocol
    try {
      const url = new URL(host);
      if (!['http:', 'https:'].includes(url.protocol)) {
        return c.json({ message: 'Only HTTP and HTTPS protocols are allowed' }, 400);
      }
    } catch {
      return c.json({ message: 'Invalid URL format' }, 400);
    }

    const response = await fetch(`${host}/api/albums`, {
      headers: { 'X-API-Key': apiKey },
    });
    const data = await response.json();
    if (Array.isArray(data)) {
      const albums = data.map((a: Record<string, unknown>) => ({ id: a.id, albumName: a.albumName }));
      return c.json(albums);
    } else {
      return c.json({ message: 'Unexpected response from Immich' }, 500);
    }
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch albums');
  }
});

// Sync metadata for a single image to Immich
app.post('/sync-metadata/:imageId', async (c) => {
  try {
    const { immichSyncService } = await import('../services/immich-sync');
    const imageId = parseInt(c.req.param('imageId'));
    if (isNaN(imageId)) {
      return c.json({ message: 'Invalid image ID' }, 400);
    }
    const result = await immichSyncService.syncImageMetadata(imageId);
    if (result.success) {
      return c.json({ message: 'Metadata synced to Immich' });
    } else {
      return c.json({ message: result.error }, 400);
    }
  } catch (error) {
    return handleRouteError(c, error, 'Failed to sync metadata');
  }
});

// Sync metadata for all images to Immich
app.post('/sync-metadata-all', async (c) => {
  try {
    const { immichSyncService } = await import('../services/immich-sync');
    const result = await immichSyncService.syncAllImages();
    return c.json({
      message: `Synced ${result.synced} images, ${result.failed} failed`,
      ...result,
    });
  } catch (error) {
    return handleRouteError(c, error, 'Failed to sync all metadata');
  }
});

// Trigger image backfill manually from admin UI
app.post('/backfill', async (c) => {
  try {
    const { runBackfill, isBackfillRunning } = await import('../workers/backfill-images');
    if (isBackfillRunning()) {
      return c.json({ message: 'Backfill already running' }, 409);
    }
    // Fire and forget — progress reported over WebSocket
    runBackfill().catch(err => console.error('[BACKFILL] Error:', err.message));
    return c.json({ message: 'Backfill started' });
  } catch (error) {
    return handleRouteError(c, error, 'Failed to start backfill');
  }
});

export default app;
