import cron, { ScheduledTask } from 'node-cron';
import { readdir, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { configService } from './config';
import { storage } from './storage';

import type { WsManager } from './ws-manager';

let wsManager: WsManager | null = null;

export function setWsManager(wm: WsManager) {
  wsManager = wm;
}

interface CronJob {
  id: string;
  name: string;
  schedule: string;
  task: ScheduledTask;
  lastRun?: Date;
  lastError?: string;
  enabled: boolean;
}

class CronManager {
  private jobs: Map<string, CronJob> = new Map();
  private isInitialized = false;

  async initialize() {
    if (this.isInitialized) return;

    console.log('[CRON] Initializing cron manager...');

    // Set up Immich sync job
    await this.setupImmichSync();

    // Clean up old notifications daily
    this.setupNotificationCleanup();

    // Sweep orphaned image directories nightly
    this.setupOrphanSweep();

    this.isInitialized = true;
    console.log('[CRON] Cron manager initialized');
  }

  private async setupImmichSync() {
    const config = await configService.getConfig();
    const cronExpr = config.immich.syncFrequency || '0 */4 * * *';

    this.scheduleJob('immich-sync', 'Immich Sync', cronExpr, async () => {
      try {
        console.log('[CRON] Starting Immich sync...');
        const { immichSyncService } = await import('./immich-sync');
        const result = await immichSyncService.syncImagesFromImmich();

        console.log(`[CRON] Immich sync completed: ${result.message}`);

        if (wsManager) {
          wsManager.broadcast('immich-sync-complete', {
            success: true,
            message: result.message,
            syncedCount: result.syncedCount,
            removedCount: result.removedCount
          });
        }

        const job = this.jobs.get('immich-sync');
        if (job) {
          job.lastRun = new Date();
          delete job.lastError;
        }

      } catch (error: unknown) {
        const err = error as Error;
        const errorMessage = err.message || 'Unknown error';
        console.error('[CRON] Immich sync failed:', errorMessage);

        if (wsManager) {
          wsManager.broadcast('immich-sync-complete', {
            success: false,
            message: errorMessage
          });
        }

        const job = this.jobs.get('immich-sync');
        if (job) {
          job.lastRun = new Date();
          job.lastError = errorMessage;
        }

        try {
          await storage.createNotification({
            type: 'error',
            title: 'Immich Sync Failed',
            message: `Automatic sync with Immich failed: ${errorMessage}`,
            details: {
              jobId: 'immich-sync',
              timestamp: new Date().toISOString(),
              error: errorMessage
            }
          });
          console.log('[CRON] Error notification created successfully');
        } catch (notificationError) {
          console.error('[CRON] Failed to create error notification:', notificationError);
        }
      }
    });
  }

  private setupNotificationCleanup() {
    this.scheduleJob('notification-cleanup', 'Notification Cleanup', '0 2 * * *', async () => {
      try {
        await storage.clearOldNotifications(30);
        console.log('[CRON] Old notifications cleaned up');
      } catch (error: unknown) {
        console.error('[CRON] Failed to clean up notifications:', (error as Error).message);
      }
    });
  }

  private setupOrphanSweep() {
    this.scheduleJob('orphan-sweep', 'Orphan Image Sweep', '0 3 * * *', async () => {
      const storagePath = process.env.STORAGE_PATH ?? (
        process.env.NODE_ENV === 'production' ? '/app/data/images' : './data/images'
      );
      const processedRoot = join(storagePath, 'processed');

      let swept = 0;

      try {
        const buckets = await readdir(processedRoot).catch(() => [] as string[]);
        for (const bucket of buckets) {
          const bucketDir = join(processedRoot, bucket);
          const imageDirs = await readdir(bucketDir).catch(() => [] as string[]);
          for (const idStr of imageDirs) {
            const imageId = parseInt(idStr, 10);
            if (isNaN(imageId)) continue;
            const row = await storage.getAstroImage(imageId).catch(() => undefined);
            if (!row) {
              await rm(join(bucketDir, idStr), { recursive: true, force: true });
              swept++;
              console.log(`[ORPHAN-SWEEP] Removed orphan dir for id=${imageId}`);
            }
          }
        }
        console.log(`[ORPHAN-SWEEP] Done: ${swept} orphans removed`);
      } catch (err: unknown) {
        console.error('[ORPHAN-SWEEP] Error during sweep:', (err as Error).message);
      }
    });
  }

  private scheduleJob(id: string, name: string, schedule: string, task: () => Promise<void>) {
    this.stopJob(id);

    try {
      const safeTask = async () => {
        try {
          await task();
        } catch (error: unknown) {
          const errMsg = (error as Error).message;
          console.error(`[CRON] Unhandled error in job ${name}:`, errMsg);

          try {
            await storage.createNotification({
              type: 'error',
              title: 'Cron Job Error',
              message: `Unhandled error in ${name}: ${errMsg}`,
              details: { jobId: id, error: errMsg }
            });
          } catch (notificationError) {
            console.error('[CRON] Failed to create unhandled error notification:', notificationError);
          }
        }
      };

      const cronTask = cron.schedule(schedule, safeTask);

      const job: CronJob = {
        id,
        name,
        schedule,
        task: cronTask,
        enabled: true
      };

      this.jobs.set(id, job);
      console.log(`[CRON] Scheduled ${name} with cron: ${schedule}`);

    } catch (error: unknown) {
      console.error(`[CRON] Failed to schedule ${name}:`, error);

      try {
        storage.createNotification({
          type: 'error',
          title: 'Cron Job Scheduling Failed',
          message: `Failed to schedule ${name} with cron expression: ${schedule}`,
          details: { jobId: id, schedule, error: (error as Error).message }
        });
      } catch (notificationError) {
        console.error('[CRON] Failed to create scheduling failure notification:', notificationError);
      }
    }
  }

  private stopJob(id: string) {
    const job = this.jobs.get(id);
    if (job) {
      job.task.stop();
      job.task.destroy();
      this.jobs.delete(id);
    }
  }

  async rescheduleJob(id: string) {
    if (id === 'immich-sync') {
      await this.setupImmichSync();
    }
  }

  async rescheduleAll() {
    console.log('[CRON] Rescheduling all jobs...');
    await this.setupImmichSync();
  }

  getJobStatus(id: string) {
    return this.jobs.get(id);
  }

  getAllJobs() {
    return Array.from(this.jobs.values());
  }

  async shutdown() {
    console.log('[CRON] Shutting down cron manager...');
    for (const job of Array.from(this.jobs.values())) {
      job.task.stop();
      job.task.destroy();
    }
    this.jobs.clear();
    this.isInitialized = false;
  }
}

export const cronManager = new CronManager();
