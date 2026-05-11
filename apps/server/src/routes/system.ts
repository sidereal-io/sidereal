import { Hono } from 'hono';
import { storage } from '../services/storage';
import { configService } from '../services/config';
import { cronManager } from '../services/cron-manager';
import { workerManager } from '../services/worker-manager';
import { astrometryService } from '../services/astrometry';
import { filterRelevantTags } from '../services/tags-utils';
import { handleRouteError } from './route-utils';
import { isPostgres, sqliteDbPath } from '../db';
import { stat, readFile } from 'node:fs/promises';
import type { WsManager } from '../services/ws-manager';

function maskApiKey(key: string): string {
  if (!key || key.length <= 4) return key ? '••••' : '';
  return '••••' + key.slice(-4);
}

function isRedactedKey(key: string): boolean {
  return key.startsWith('••••');
}

export default function systemRoutes(wsManager?: WsManager) {
  const app = new Hono();

  // Save admin settings
  app.post('/admin/settings', async (c) => {
    try {
      const settings = await c.req.json();

      // Validate Immich host URL if provided
      if (settings.immich?.host) {
        try {
          const url = new URL(settings.immich.host);
          if (!['http:', 'https:'].includes(url.protocol)) {
            return c.json({
              message: 'Only HTTP and HTTPS protocols are allowed for Immich host',
            }, 400);
          }
        } catch {
          return c.json({
            message: 'Invalid Immich host URL format',
          }, 400);
        }
      }

      // Preserve existing API keys if the client sends back redacted values
      if (settings.immich?.apiKey && isRedactedKey(settings.immich.apiKey)) {
        const currentConfig = await configService.getConfig();
        settings.immich.apiKey = currentConfig.immich.apiKey;
      }
      if (settings.astrometry?.apiKey && isRedactedKey(settings.astrometry.apiKey)) {
        const currentConfig = await configService.getConfig();
        settings.astrometry.apiKey = currentConfig.astrometry.apiKey;
      }

      await configService.updateConfig(settings);
      return c.json({ message: 'Settings saved successfully' });
    } catch (error) {
      return handleRouteError(c, error, 'Failed to save settings');
    }
  });

  // Get admin settings (with redacted API keys)
  app.get('/admin/settings', async (c) => {
    try {
      const config = await configService.getConfig();
      const redacted = {
        ...config,
        immich: {
          ...config.immich,
          apiKey: maskApiKey(config.immich.apiKey),
        },
        astrometry: {
          ...config.astrometry,
          apiKey: maskApiKey(config.astrometry.apiKey),
        },
      };
      return c.json(redacted);
    } catch (error) {
      return handleRouteError(c, error, 'Failed to get settings');
    }
  });

  // Test Astrometry connection
  app.post('/test-astrometry-connection', async (c) => {
    try {
      let { apiKey } = await c.req.json();

      // If a redacted key was sent, use the stored key
      if (apiKey && isRedactedKey(apiKey)) {
        const config = await configService.getConfig();
        apiKey = config.astrometry.apiKey;
      }

      if (!apiKey) {
        return c.json({ message: 'API key is required' }, 400);
      }

      // Test the connection by trying to login (same as working plate solve)
      const loginData = new URLSearchParams();
      loginData.append('request-json', JSON.stringify({ apikey: apiKey }));

      const response = await fetch('https://nova.astrometry.net/api/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: loginData,
        signal: AbortSignal.timeout(10000),
      });

      const data = await response.json() as Record<string, unknown>;

      if (response.ok && data.status === 'success') {
        return c.json({ success: true, message: 'Connection successful!' });
      } else {
        return c.json({
          success: false, message: `Connection failed: ${data.message || 'Unknown error'}`,
        });
      }
    } catch (error: unknown) {
      const err = error as Error & { code?: string };

      let errorMessage = 'Connection failed';
      if (err.code === 'ECONNREFUSED') {
        errorMessage = 'Cannot connect to Astrometry.net server.';
      } else if (err.code === 'ENOTFOUND') {
        errorMessage = 'Astrometry.net server not found.';
      } else if (err.message) {
        errorMessage = err.message;
      }

      return c.json({ message: errorMessage }, 500);
    }
  });

  // Get stats
  app.get('/stats', async (c) => {
    try {
      const stats = await storage.getStats();
      return c.json(stats);
    } catch (error) {
      return handleRouteError(c, error, 'Failed to fetch stats');
    }
  });

  // Get popular tags
  app.get('/tags', async (c) => {
    try {
      const images = await storage.getAstroImages();
      const tagCounts: Record<string, number> = {};

      images.forEach((image) => {
        if (Array.isArray(image.tags)) {
          const relevantTags = filterRelevantTags(image.tags);
          relevantTags.forEach((tag) => {
            tagCounts[tag] = (tagCounts[tag] || 0) + 1;
          });
        }
      });

      const popularTags = Object.entries(tagCounts)
        .sort(([, a], [, b]) => b - a)
        .slice(0, 20)
        .map(([tag, count]) => ({ tag, count }));

      return c.json(popularTags);
    } catch (error) {
      return handleRouteError(c, error, 'Failed to fetch tags');
    }
  });

  // Get constellations
  app.get('/constellations', async (c) => {
    try {
      const images = await storage.getAstroImages();
      const constellations = Array.from(
        new Set(images.filter((img) => img.constellation).map((img) => img.constellation!)),
      ).sort();
      return c.json(constellations);
    } catch (error) {
      return handleRouteError(c, error, 'Failed to fetch constellations');
    }
  });

  // Get notifications
  app.get('/notifications', async (c) => {
    try {
      const notifications = await storage.getNotifications();
      return c.json(notifications);
    } catch (error) {
      return handleRouteError(c, error, 'Failed to get notifications');
    }
  });

  // Acknowledge all notifications
  app.post('/notifications/acknowledge-all', async (c) => {
    try {
      await storage.acknowledgeAllNotifications();
      if (wsManager) {
        wsManager.broadcast('notifications-updated', {});
      }
      return c.json({ message: 'All notifications acknowledged' });
    } catch (error) {
      return handleRouteError(c, error, 'Failed to acknowledge all notifications');
    }
  });

  // Acknowledge notification
  app.post('/notifications/:id/acknowledge', async (c) => {
    try {
      const id = parseInt(c.req.param('id'));
      await storage.acknowledgeNotification(id);
      if (wsManager) {
        wsManager.broadcast('notifications-updated', {});
      }
      return c.json({ message: 'Notification acknowledged' });
    } catch (error) {
      return handleRouteError(c, error, 'Failed to acknowledge notification');
    }
  });

  // Get cron job status
  app.get('/admin/cron-jobs', async (c) => {
    try {
      const jobs = cronManager.getAllJobs();
      return c.json(jobs);
    } catch (error) {
      return handleRouteError(c, error, 'Failed to get cron jobs');
    }
  });

  // Database info (type, size, path)
  app.get('/admin/database', async (c) => {
    try {
      const dbType = isPostgres ? 'postgresql' : 'sqlite';
      const info: Record<string, unknown> = { type: dbType };

      if (!isPostgres && sqliteDbPath) {
        info.path = sqliteDbPath;
        try {
          const stats = await stat(sqliteDbPath);
          info.sizeBytes = stats.size;
          info.lastModified = stats.mtime.toISOString();
        } catch {
          info.sizeBytes = 0;
        }
      }

      return c.json(info);
    } catch (error) {
      return handleRouteError(c, error, 'Failed to get database info');
    }
  });

  // Download SQLite database backup
  app.get('/admin/database/backup', async (c) => {
    try {
      if (isPostgres) {
        return c.json({ message: 'Backup download is only available for SQLite databases. Use pg_dump for PostgreSQL.' }, 400);
      }
      if (!sqliteDbPath) {
        return c.json({ message: 'SQLite database path not configured' }, 500);
      }

      const dbBuffer = await readFile(sqliteDbPath);
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
      const filename = `sidereal-backup-${timestamp}.db`;

      return new Response(dbBuffer, {
        headers: {
          'Content-Type': 'application/x-sqlite3',
          'Content-Disposition': `attachment; filename="${filename}"`,
          'Content-Length': String(dbBuffer.length),
        },
      });
    } catch (error) {
      return handleRouteError(c, error, 'Failed to create database backup');
    }
  });

  // Health check endpoint for container monitoring
  app.get('/health', async (c) => {
    try {
      // Check database connection
      let databaseStatus = 'healthy';
      try {
        await storage.getStats();
      } catch (dbError) {
        console.error('Database health check failed:', dbError);
        databaseStatus = 'unhealthy';
      }

      // Get worker status
      const workerStatus = workerManager.getStatus();

      const healthStatus = {
        status: databaseStatus === 'healthy' ? 'healthy' : 'unhealthy',
        timestamp: new Date().toISOString(),
        uptime: process.uptime(),
        database: databaseStatus,
        worker: {
          enabled: workerStatus.enabled,
          running: workerStatus.running,
          pid: workerStatus.pid,
          restartAttempts: workerStatus.restartAttempts,
        },
        version: process.env.npm_package_version || 'unknown',
        nodeVersion: process.version,
      };

      const statusCode = databaseStatus === 'healthy' ? 200 : 503;
      return c.json(healthStatus, statusCode as 200);
    } catch (error) {
      console.error('Health check error:', error);
      return c.json({
        status: 'unhealthy',
        timestamp: new Date().toISOString(),
        error: 'Health check failed',
      }, 503);
    }
  });

  return app;
}
