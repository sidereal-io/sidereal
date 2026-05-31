import { Hono } from 'hono';
import { readFile } from 'fs/promises';
import { stat } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { storage } from '../services/storage';
import { xmpSidecarService } from '../services/xmp-sidecar';
import { configService } from '../services/config';
import { handleRouteError } from './route-utils';
import { imageStorage, ImageNotFoundError } from '../services/image-storage';

const MIME: Record<string, string> = {
  jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png',
  gif: 'image/gif', webp: 'image/webp', tiff: 'image/tiff',
  fit: 'application/octet-stream', xisf: 'application/octet-stream',
};

function extMime(filePath: string): string {
  const ext = filePath.split('.').pop()?.toLowerCase() ?? '';
  return MIME[ext] ?? 'application/octet-stream';
}

const app = new Hono();

// Helper to update Immich asset metadata
async function updateImmichAssetMetadata(immichId: string, metadata: { latitude?: number; longitude?: number }) {
  try {
    const config = await configService.getImmichConfig();
    if (!config.host || !config.apiKey) {
      console.warn('Immich config missing, skipping metadata sync');
      return;
    }

    await fetch(`${config.host}/api/assets`, {
      method: 'PUT',
      headers: { 'X-API-Key': config.apiKey, 'Content-Type': 'application/json' },
      body: JSON.stringify({
        ids: [immichId],
        latitude: metadata.latitude,
        longitude: metadata.longitude,
      }),
    });
    console.log(`Synced metadata to Immich for asset ${immichId}`);
  } catch (error: unknown) {
    const err = error as Error;
    console.error(`Failed to sync metadata to Immich for asset ${immichId}:`, err.message);
  }
}

// Get all astrophotography images with optional filters
app.get('/', async (c) => {
  try {
    const objectType = c.req.query('objectType');
    const tags = c.req.queries('tags');
    const plateSolved = c.req.query('plateSolved');
    const constellation = c.req.query('constellation');
    const equipmentId = c.req.query('equipmentId');
    const filters: { objectType?: string; tags?: string[]; plateSolved?: boolean; constellation?: string; equipmentId?: number } = {};

    if (objectType) filters.objectType = objectType;
    if (tags && tags.length > 0) filters.tags = tags;
    if (plateSolved !== undefined) filters.plateSolved = plateSolved === 'true';
    if (constellation) filters.constellation = constellation;
    if (equipmentId) filters.equipmentId = parseInt(equipmentId);

    const images = await storage.getAstroImages(filters);
    return c.json(images);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch images');
  }
});

// --- Image byte-serving routes ---

app.get('/:id/thumbnail', async (c) => {
  return serveFile(c, parseInt(c.req.param('id')), 'thumbnail');
});

app.get('/:id/preview', async (c) => {
  return serveFile(c, parseInt(c.req.param('id')), 'preview');
});

app.get('/:id/original', async (c) => {
  return serveFile(c, parseInt(c.req.param('id')), 'original', true);
});

// Get a specific image
app.get('/:id', async (c) => {
  try {
    const id = parseInt(c.req.param('id'));
    const image = await storage.getAstroImage(id);
    if (!image) {
      return c.json({ message: 'Image not found' }, 404);
    }
    return c.json(image);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch image');
  }
});

// Update a specific image
app.patch('/:id', async (c) => {
  try {
    const id = parseInt(c.req.param('id'));
    const updates = await c.req.json();

    const updatedImage = await storage.updateAstroImage(id, updates);
    if (!updatedImage) {
      return c.json({ message: 'Image not found' }, 404);
    }

    // Sync to Immich if coordinates were updated
    if (updatedImage.immichId && (updates.latitude !== undefined || updates.longitude !== undefined)) {
      await updateImmichAssetMetadata(updatedImage.immichId, {
        latitude: updatedImage.latitude || undefined,
        longitude: updatedImage.longitude || undefined,
      });
    }

    return c.json(updatedImage);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to update image');
  }
});

// Get plate solving job data for an image
app.get('/:id/plate-solving-job', async (c) => {
  try {
    const imageId = parseInt(c.req.param('id'));
    const image = await storage.getAstroImage(imageId);

    if (!image) {
      return c.json({ message: 'Image not found' }, 404);
    }

    // Find the successful plate solving job for this image directly
    const job = await storage.getLatestPlateSolvingJob(imageId, 'success');

    if (!image.plateSolved || !job) {
      return c.json({ message: 'Image has not been successfully plate solved' }, 400);
    }

    return c.json({
      jobId: job.id,
      submissionId: job.astrometrySubmissionId,
      astrometryJobId: job.astrometryJobId,
      status: job.status,
      submittedAt: job.submittedAt,
      completedAt: job.completedAt,
    });
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch plate solving job data');
  }
});

// Get image annotations from stored plate solving data
app.get('/:id/annotations', async (c) => {
  try {
    const imageId = parseInt(c.req.param('id'));
    const image = await storage.getAstroImage(imageId);

    if (!image) {
      return c.json({ message: 'Image not found' }, 404);
    }

    // Find the successful plate solving job for this image directly
    const job = await storage.getLatestPlateSolvingJob(imageId, 'success');

    if (!image.plateSolved || !job) {
      return c.json({ message: 'Image has not been plate solved' }, 400);
    }

    if (!job.result) {
      return c.json({ message: 'No successful plate solving data found' }, 400);
    }

    const result = job.result as Record<string, unknown>;
    const annotations = (result.annotations as unknown[]) || [];
    const calibration = {
      ra: result.ra,
      dec: result.dec,
      pixscale: result.pixscale,
      radius: result.radius,
      orientation: result.orientation,
    };

    return c.json({
      annotations: annotations,
      calibration: calibration,
      imageDimensions: {
        width: result.width || null,
        height: result.height || null,
      },
    });
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch image annotations');
  }
});

// Get equipment for a specific image
app.get('/:id/equipment', async (c) => {
  try {
    const imageId = parseInt(c.req.param('id'));
    const image = await storage.getAstroImage(imageId);

    if (!image) {
      return c.json({ message: 'Image not found' }, 404);
    }

    const equipment = await storage.getEquipmentForImage(imageId);
    const imageEquipment = await storage.getImageEquipment(imageId);

    // Combine equipment with their relationship data
    const equipmentWithDetails = equipment.map((eq) => {
      const relationship = imageEquipment.find((ie) => ie.equipmentId === eq.id);
      return {
        ...eq,
        settings: relationship?.settings || null,
        notes: relationship?.notes || null,
      };
    });

    return c.json(equipmentWithDetails);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch image equipment');
  }
});

// Add equipment to an image
app.post('/:id/equipment', async (c) => {
  try {
    const imageId = parseInt(c.req.param('id'));
    const { equipmentId, settings, notes } = await c.req.json();

    const image = await storage.getAstroImage(imageId);
    if (!image) {
      return c.json({ message: 'Image not found' }, 404);
    }

    const equipment = await storage.getEquipment();
    const targetEquipment = equipment.find((eq) => eq.id === equipmentId);
    if (!targetEquipment) {
      return c.json({ message: 'Equipment not found' }, 404);
    }

    const imageEquipment = await storage.addEquipmentToImage(imageId, equipmentId, settings, notes);
    return c.json(imageEquipment);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to add equipment to image');
  }
});

// Remove equipment from an image
app.delete('/:imageId/equipment/:equipmentId', async (c) => {
  try {
    const imageId = parseInt(c.req.param('imageId'));
    const equipmentId = parseInt(c.req.param('equipmentId'));

    const success = await storage.removeEquipmentFromImage(imageId, equipmentId);
    if (!success) {
      return c.json({ message: 'Equipment relationship not found' }, 404);
    }

    return c.json({ message: 'Equipment removed from image' });
  } catch (error) {
    return handleRouteError(c, error, 'Failed to remove equipment from image');
  }
});

// Update equipment settings for an image
app.put('/:imageId/equipment/:equipmentId', async (c) => {
  try {
    const imageId = parseInt(c.req.param('imageId'));
    const equipmentId = parseInt(c.req.param('equipmentId'));
    const { settings, notes } = await c.req.json();

    const imageEquipment = await storage.updateImageEquipment(imageId, equipmentId, { settings, notes });
    if (!imageEquipment) {
      return c.json({ message: 'Equipment relationship not found' }, 404);
    }

    return c.json(imageEquipment);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to update equipment settings');
  }
});

// --- Image Acquisition Routes ---

// Get acquisition entries for an image
app.get('/:id/acquisitions', async (c) => {
  try {
    const imageId = parseInt(c.req.param('id'));
    const acquisitions = await storage.getImageAcquisitions(imageId);
    return c.json(acquisitions);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch acquisition entries');
  }
});

// Add acquisition entry to an image
app.post('/:id/acquisitions', async (c) => {
  try {
    const imageId = parseInt(c.req.param('id'));
    const { filterId, filterName, frameCount, exposureTime, gain, offset, binning, sensorTemp, date, notes } = await c.req.json();

    if (!frameCount || !exposureTime) {
      return c.json({ message: 'frameCount and exposureTime are required' }, 400);
    }

    const acquisition = await storage.createImageAcquisition({
      imageId,
      filterId: filterId || null,
      filterName: filterName || null,
      frameCount,
      exposureTime,
      gain: gain ?? null,
      offset: offset ?? null,
      binning: binning || null,
      sensorTemp: sensorTemp ?? null,
      date: date ? new Date(date) : null,
      notes: notes || null,
    });
    return c.json(acquisition);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to create acquisition entry');
  }
});

// Update an acquisition entry
app.put('/:imageId/acquisitions/:id', async (c) => {
  try {
    const id = parseInt(c.req.param('id'));
    const { filterId, filterName, frameCount, exposureTime, gain, offset, binning, sensorTemp, date, notes } = await c.req.json();

    const acquisition = await storage.updateImageAcquisition(id, {
      filterId, filterName, frameCount, exposureTime,
      gain, offset, binning, sensorTemp,
      date: date ? new Date(date) : null,
      notes,
    });

    if (!acquisition) {
      return c.json({ message: 'Acquisition entry not found' }, 404);
    }
    return c.json(acquisition);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to update acquisition entry');
  }
});

// Delete an acquisition entry
app.delete('/:imageId/acquisitions/:id', async (c) => {
  try {
    const id = parseInt(c.req.param('id'));
    const success = await storage.deleteImageAcquisition(id);
    if (!success) {
      return c.json({ message: 'Acquisition entry not found' }, 404);
    }
    return c.json({ message: 'Acquisition entry deleted' });
  } catch (error) {
    return handleRouteError(c, error, 'Failed to delete acquisition entry');
  }
});

// Get XMP sidecar for an image
app.get('/:id/sidecar', async (c) => {
  try {
    const id = parseInt(c.req.param('id'));
    const image = await storage.getAstroImage(id);

    if (!image) {
      return c.json({ message: 'Image not found' }, 404);
    }

    const sidecarConfig = await configService.getSidecarConfig();
    const sidecarPath = await xmpSidecarService.resolveSidecarPath(image, sidecarConfig);

    if (!sidecarPath) {
      return c.json({ message: 'No XMP sidecar found for this image' }, 404);
    }

    const content = await readFile(sidecarPath, 'utf8');
    const isDownload = c.req.query('download') === 'true';

    const headers: Record<string, string> = { 'Content-Type': 'application/xml' };
    if (isDownload) {
      headers['Content-Disposition'] = `attachment; filename="${image.filename}.xmp"`;
    }

    return new Response(content, { headers });
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch sidecar');
  }
});

async function serveFile(
  c: any,
  id: number,
  size: 'original' | 'preview' | 'thumbnail',
  rangeSupport = false
): Promise<Response> {
  let filePath: string;
  try {
    filePath = await imageStorage.readPath(id, size);
  } catch (e) {
    if (e instanceof ImageNotFoundError) {
      return c.json({ message: 'Image file not found', backfillPending: true }, 404, {
        'X-Sidereal-Backfill': 'pending',
      });
    }
    throw e;
  }

  let fileStats: { size: number; mtime: Date };
  try {
    fileStats = await stat(filePath);
  } catch {
    return c.json({ message: 'Image file not found' }, 404);
  }

  const contentType = extMime(filePath);
  const etag = `W/"${id}-${size}-${fileStats.mtime.getTime()}"`;
  const ifNoneMatch = c.req.header('If-None-Match');

  if (ifNoneMatch === etag) {
    return new Response(null, { status: 304 });
  }

  const baseHeaders: Record<string, string> = {
    'Content-Type': contentType,
    'Cache-Control': 'public, max-age=31536000, immutable',
    'ETag': etag,
  };

  if (c.req.query('download') === '1') {
    const filename = filePath.split('/').pop() ?? `image-${id}`;
    baseHeaders['Content-Disposition'] = `attachment; filename="${filename}"`;
  }

  if (rangeSupport) {
    const rangeHeader = c.req.header('Range');
    if (rangeHeader) {
      const match = rangeHeader.match(/^bytes=(\d+)-(\d*)$/);
      if (!match) {
        return new Response(null, { status: 416, headers: { 'Content-Range': `bytes */${fileStats.size}` } });
      }
      const start = parseInt(match[1]);
      const end = match[2] ? parseInt(match[2]) : fileStats.size - 1;

      if (start >= fileStats.size || end >= fileStats.size || start > end) {
        return new Response(null, { status: 416, headers: { 'Content-Range': `bytes */${fileStats.size}` } });
      }

      const chunkSize = end - start + 1;
      const stream = createReadStream(filePath, { start, end });
      return new Response(stream as any, {
        status: 206,
        headers: {
          ...baseHeaders,
          'Content-Length': String(chunkSize),
          'Content-Range': `bytes ${start}-${end}/${fileStats.size}`,
          'Accept-Ranges': 'bytes',
        },
      });
    }
  }

  const stream = createReadStream(filePath);
  return new Response(stream as any, {
    status: 200,
    headers: {
      ...baseHeaders,
      'Content-Length': String(fileStats.size),
      'Accept-Ranges': rangeSupport ? 'bytes' : 'none',
    },
  });
}

export default app;
