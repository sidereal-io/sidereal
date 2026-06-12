import { Hono } from 'hono';
import { handleRouteError } from './route-utils.js';
import { sourceRegistry } from '../services/source-registry.js';
import { localUploadSource } from '../services/sources/local-upload.js';
import { urlUploadSource } from '../services/sources/url-upload.js';
import { MAX_ORIGINAL_BYTES } from '../services/image-storage.js';

const app = new Hono();

app.get('/', async (c) => {
  try {
    const sources = sourceRegistry.list();
    const statuses = await Promise.all(
      sources.map(async (s) => {
        const status = await s.getStatus();
        return { displayName: s.displayName, ...status };
      }),
    );
    return c.json(statuses);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to list sources');
  }
});

app.post('/upload', async (c) => {
  try {
    const body = await c.req.parseBody();
    const file = body['file'];
    if (!file || !(file instanceof File)) {
      return c.json({ message: 'No file provided' }, 400);
    }
    const bytes = Buffer.from(await file.arrayBuffer());
    if (bytes.byteLength > MAX_ORIGINAL_BYTES) {
      return c.json({ message: 'File exceeds 500 MB limit' }, 413);
    }
    const result = await localUploadSource.ingest(bytes, file.name);
    return c.json(result, 201);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to upload image');
  }
});

app.post('/ingest-url', async (c) => {
  try {
    const body = await c.req.json() as Record<string, unknown>;
    const url = body['url'];
    if (typeof url !== 'string' || !url) {
      return c.json({ message: 'url is required' }, 400);
    }
    let parsed: URL;
    try {
      parsed = new URL(url);
    } catch {
      return c.json({ message: 'Invalid URL' }, 400);
    }
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
      return c.json({ message: 'Only http and https URLs are supported' }, 400);
    }
    const result = await urlUploadSource.ingest(url);
    return c.json(result, 201);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to ingest image from URL');
  }
});

app.get('/:type/status', async (c) => {
  try {
    const sourceType = c.req.param('type');
    const plugin = sourceRegistry.get(sourceType);
    if (!plugin) {
      return c.json({ message: `Unknown source type: ${sourceType}` }, 404);
    }
    const status = await plugin.getStatus();
    return c.json(status);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to get source status');
  }
});

app.post('/:type/test', async (c) => {
  try {
    const sourceType = c.req.param('type');
    const plugin = sourceRegistry.get(sourceType);
    if (!plugin) {
      return c.json({ message: `Unknown source type: ${sourceType}` }, 404);
    }
    const result = await plugin.testConnection();
    return c.json(result);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to test source connection');
  }
});

export default app;
