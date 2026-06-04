import { Hono } from 'hono';
import { storage } from '../services/storage';
import { handleRouteError } from './route-utils';
import { imageUrl } from '@shared/utils';

const app = new Hono();

// Get plate-solved images as sky map markers
app.get('/markers', async (c) => {
  try {
    const images = await storage.getAstroImages({ plateSolved: true });

    const markers = images
      .filter((img) => img.ra && img.dec)
      .map((img) => ({
        id: img.id,
        title: img.title,
        ra: img.ra,
        dec: img.dec,
        thumbnailUrl: img.id ? imageUrl(img.id, 'thumbnail') : null,
        objectType: img.objectType,
        constellation: img.constellation,
        fieldOfView: img.fieldOfView,
      }));

    return c.json(markers);
  } catch (error) {
    return handleRouteError(c, error, 'Failed to fetch sky map markers');
  }
});

export default app;
