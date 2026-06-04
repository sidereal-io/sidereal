import { storage } from "../services/storage";
import { AstrometryService } from "../services/astrometry";
import { configService } from "../services/config";
import WebSocket from "ws";

class PlateSolvingWorker {
  private isRunning = false;
  private checkInterval = 30000;
  private maxConcurrent: number = 3;
  private astrometryService: AstrometryService;
  private ws: WebSocket | null = null;
  private reconnectDelay = 1000;
  private maxReconnectDelay = 30000;

  constructor() {
    this.astrometryService = new AstrometryService(true);
    this.connectToServer();
  }

  private connectToServer() {
    try {
      this.ws = new WebSocket('ws://localhost:5000/ws');

      this.ws.on('open', () => {
        console.log('Worker connected to server via WebSocket');
        this.reconnectDelay = 1000;
      });

      this.ws.on('close', () => {
        console.log('Worker disconnected from server');
        this.scheduleReconnect();
      });

      this.ws.on('error', (error) => {
        console.error('Worker WebSocket error:', error.message);
      });
    } catch (error) {
      console.error('Failed to connect worker to server:', error);
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect() {
    setTimeout(() => {
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
      this.connectToServer();
    }, this.reconnectDelay);
  }

  private emitUpdate(event: string, data: unknown) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ event, data }));
    }
  }

  async start() {
    if (this.isRunning) {
      console.log("Worker is already running");
      return;
    }

    const astrometryConfig = await this.getWorkerConfig();
    this.checkInterval = astrometryConfig.checkInterval * 1000;
    this.maxConcurrent = astrometryConfig.maxConcurrent;

    this.isRunning = true;
    console.log(`Starting plate solving worker with max ${this.maxConcurrent} concurrent jobs, check interval: ${astrometryConfig.checkInterval}s`);

    while (this.isRunning) {
      try {
        const currentConfig = await this.getWorkerConfig();
        this.checkInterval = currentConfig.checkInterval * 1000;
        this.maxConcurrent = currentConfig.maxConcurrent;

        await this.createAndSubmitJobsForUnsolvedImages();
        await this.checkProcessingJobs();
        await this.sleep(this.checkInterval);
      } catch (error) {
        console.error("Worker error:", error);
        await this.sleep(5000);
      }
    }
  }

  private async isStandaloneMode(): Promise<boolean> {
    try {
      const astrometryConfig = await configService.getAstrometryConfig();
      return !astrometryConfig.apiKey && !!(process.env.ASTROMETRY_API_KEY || process.env.ASTROMETRY_KEY);
    } catch (error) {
      return true;
    }
  }

  private async getWorkerConfig() {
    const isStandalone = await this.isStandaloneMode();

    if (isStandalone) {
      console.log('Worker running in standalone mode - using environment variables');
      return {
        apiKey: process.env.ASTROMETRY_API_KEY || process.env.ASTROMETRY_KEY || "",
        enabled: true,
        autoEnabled: true,
        checkInterval: parseInt(process.env.ASTROMETRY_CHECK_INTERVAL || "30", 10),
        pollInterval: parseInt(process.env.ASTROMETRY_POLL_INTERVAL || "5", 10),
        maxConcurrent: parseInt(process.env.PLATE_SOLVE_MAX_CONCURRENT || "3", 10),
        autoResubmit: process.env.ASTROMETRY_AUTO_RESUBMIT === 'true',
      };
    } else {
      console.log('Worker running with UI server - using database configuration');
      return await configService.getAstrometryConfig();
    }
  }

  async createAndSubmitJobsForUnsolvedImages() {
    const astrometryConfig = await this.getWorkerConfig();

    if (!astrometryConfig.enabled || !astrometryConfig.autoEnabled || !astrometryConfig.apiKey) {
      console.log('Automatic plate solving not enabled or configured, skipping job creation');
      return;
    }

    const images = await storage.getAstroImages();
    const jobs = await storage.getPlateSolvingJobs();
    const unsolvedImages = images.filter(img => !img.plateSolved);
    const processingJobs = jobs.filter(job => ["pending", "processing"].includes(job.status));

    if (processingJobs.length >= this.maxConcurrent) {
      console.log(`Max concurrent plate solving jobs (${this.maxConcurrent}) reached.`);
      return;
    }

    const slotsAvailable = this.maxConcurrent - processingJobs.length;
    let submitted = 0;

    for (const image of unsolvedImages) {
      if (submitted >= slotsAvailable) break;

      const existingJob = jobs.find(job => job.imageId === image.id);
      if (existingJob) {
        if (existingJob.status === "failed" && !astrometryConfig.autoResubmit) {
          continue;
        }
        if (["pending", "processing", "success"].includes(existingJob.status)) {
          continue;
        }
      }

      try {
        console.log(`Auto-submitting image ${image.id} (${image.title}) for plate solving`);
        await this.astrometryService.submitImageForPlateSolving(image);
        submitted++;
      } catch (err) {
        console.error(`Failed to auto-submit image ${image.id}:`, err);
      }
    }
  }

  async checkProcessingJobs() {
    const jobs = await storage.getPlateSolvingJobs();
    const processingJobs = jobs.filter(job => job.status === "processing");

    if (processingJobs.length > 0) {
      console.log(`Checking ${processingJobs.length} processing jobs`);
    }

    for (const job of processingJobs) {
      try {
        const result = await this.astrometryService.checkJobStatus(job.id);

        if (result.status !== "processing") {
          this.emitUpdate('plate-solving-update', {
            jobId: job.id,
            status: result.status,
            result: result.result
          });
        }
      } catch (error) {
        console.error(`Failed to update job ${job.id}:`, error);

        this.emitUpdate('plate-solving-update', {
          jobId: job.id,
          status: "error",
          error: error instanceof Error ? error.message : "Unknown error"
        });
      }
    }
  }

  private sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  stop() {
    this.isRunning = false;
    if (this.ws) {
      this.ws.close();
    }
    console.log("Stopping plate solving worker");
  }
}

export { PlateSolvingWorker };

// Start the worker
const worker = new PlateSolvingWorker();

// Handle graceful shutdown
process.on('SIGINT', () => {
  console.log("Shutting down worker...");
  worker.stop();
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.log("Shutting down worker...");
  worker.stop();
  process.exit(0);
});

// Start the worker
worker.start().catch((error) => {
  console.error("Worker failed to start:", error);
  process.exit(1);
});
