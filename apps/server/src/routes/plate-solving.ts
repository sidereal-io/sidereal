import { Hono } from 'hono';
import { storage } from '../services/storage';
import { astrometryService } from '../services/astrometry';
import { configService } from '../services/config';
import { handleRouteError } from './route-utils';
import type { WsManager } from '../services/ws-manager';

export default (wsManager?: WsManager) => {
  const app = new Hono();

  // Submit image for plate solving
  app.post('/images/:id/plate-solve', async (c) => {
    try {
      // Check if plate solving is enabled and API key is configured
      const astrometryConfig = await configService.getAstrometryConfig();
      if (!astrometryConfig.enabled || !astrometryConfig.apiKey) {
        return c.json({
          message: 'Plate solving is not configured. Please enable it and provide an API key in the admin settings.',
        }, 400);
      }

      const imageId = parseInt(c.req.param('id'));
      const image = await storage.getAstroImage(imageId);
      if (!image) {
        return c.json({ message: 'Image not found' }, 404);
      }

      // Use the shared service to complete the full plate solving workflow
      const outcome = await astrometryService.completePlateSolvingWorkflow(image);
      if (outcome.status === 'processing') return c.json(outcome, 202);

      return c.json({
        message: 'Image plate solving completed successfully',
        result: {
          calibration: outcome.result.calibration,
          annotations: outcome.result.annotations,
          machineTags: outcome.result.machineTags,
        },
      });
    } catch (error) {
      return handleRouteError(c, error, 'Failed to complete plate solving');
    }
  });

  // Check plate solving status and update results
  app.get('/jobs', async (c) => {
    try {
      const jobs = await storage.getPlateSolvingJobs();
      return c.json(jobs);
    } catch (error) {
      return handleRouteError(c, error, 'Failed to fetch plate solving jobs');
    }
  });

  // Bulk submit images for plate solving
  app.post('/bulk', async (c) => {
    try {
      const { imageIds } = await c.req.json();

      if (!Array.isArray(imageIds) || imageIds.length === 0) {
        return c.json({ message: 'imageIds array is required' }, 400);
      }

      // Check if plate solving is enabled and API key is configured
      const astrometryConfig = await configService.getAstrometryConfig();
      if (!astrometryConfig.enabled || !astrometryConfig.apiKey) {
        return c.json({
          message: 'Plate solving is not configured. Please enable it and provide an API key in the admin settings.',
        }, 400);
      }

      const results = [];

      for (const imageId of imageIds) {
        try {
          const image = await storage.getAstroImage(imageId);
          if (!image) {
            results.push({ imageId, success: false, error: 'Image not found' });
            continue;
          }

          if (image.plateSolved) {
            results.push({ imageId, success: false, error: 'Image already plate solved' });
            continue;
          }

          // Submit for plate solving (just submit, don't wait for completion)
          const { submissionId, jobId } = await astrometryService.submitImageForPlateSolving(image);

          results.push({
            imageId,
            success: true,
            submissionId,
            jobId,
            message: 'Submitted for plate solving',
          });
        } catch (error: unknown) {
          const err = error as Error & { response?: { data?: { message?: string } } };
          results.push({
            imageId,
            success: false,
            error: err.response?.data?.message || err.message,
          });
        }
      }

      return c.json({
        message: 'Bulk plate solving submission completed',
        results,
      });
    } catch (error) {
      return handleRouteError(c, error, 'Failed to submit images for plate solving');
    }
  });

  // Update plate solving job status
  app.post('/update/:jobId', async (c) => {
    try {
      const jobId = parseInt(c.req.param('jobId'));
      const { status, result } = await astrometryService.checkJobStatus(jobId, { resumeProcessing: true });

      // Emit real-time update via WebSocket if available
      if (wsManager) {
        wsManager.broadcast('plate-solving-update', { jobId, status, result });
      }

      return c.json({ status, result });
    } catch (error) {
      return handleRouteError(c, error, 'Failed to update plate solving status');
    }
  });

  return app;
};
