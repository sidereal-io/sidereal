import { storage } from './storage';
import { xmpSidecarService } from './xmp-sidecar';
import { configService } from './config';
import { getConstellationFromCoordinates } from './constellation-utils';
import { filterRelevantTags } from './tags-utils';
import { catalogService } from './catalog';
import { imageStorage } from './image-storage';
import { readFile } from 'node:fs/promises';

import type { WsManager } from './ws-manager';

let wsManager: WsManager | null = null;

export function setWsManager(wm: WsManager) {
  wsManager = wm;
}

export interface AstrometryCalibration {
  ra: number;
  dec: number;
  pixscale: number;
  radius: number;
  orientation: number;
  width_arcsec?: number;
  height_arcsec?: number;
  parity?: number;
}

export interface AstrometryAnnotation {
  type: string;
  names: string[];
  pixelx: number;
  pixely: number;
  radius?: number;
  ra?: number;
  dec?: number;
  vmag?: number;
  pixelX?: number;
  pixelY?: number;
}

export interface PlateSolvingResult {
  calibration: AstrometryCalibration;
  annotations: AstrometryAnnotation[];
  machineTags: string[];
}

const HEADERS = { 'User-Agent': 'Mozilla/5.0 (compatible; Sidereal/1.0)' };

export class AstrometryService {
  private astrometryApiKey: string;
  private immichApiKey: string;
  private immichHost: string;
  private useConfigService: boolean;

  constructor(useConfigService: boolean = true) {
    this.useConfigService = useConfigService;
    this.astrometryApiKey = "";
    this.immichApiKey = "";
    this.immichHost = "";
  }

  private async ensureConfigLoaded() {
    if (this.useConfigService && (!this.astrometryApiKey || !this.immichApiKey || !this.immichHost)) {
      const config = await configService.getConfig();
      this.astrometryApiKey = config.astrometry.apiKey;
      this.immichApiKey = config.immich.apiKey;
      this.immichHost = config.immich.host;
    }
  }

  private async login(): Promise<string> {
    await this.ensureConfigLoaded();

    if (!this.astrometryApiKey) {
      throw new Error("Astrometry.net API key not configured");
    }

    const loginData = new URLSearchParams();
    loginData.append('request-json', JSON.stringify({ apikey: this.astrometryApiKey }));

    const loginResponse = await fetch("https://nova.astrometry.net/api/login", {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        ...HEADERS,
      },
      body: loginData,
    });

    const loginResult = await loginResponse.json() as Record<string, unknown>;

    if (loginResult.status !== "success") {
      throw new Error("Failed to authenticate with Astrometry.net");
    }

    return loginResult.session as string;
  }

  async submitImageForPlateSolving(image: { id: number; immichId?: string | null; title?: string | null; filename?: string | null }): Promise<{ submissionId: string; jobId: number }> {
    await this.ensureConfigLoaded();

    const sessionKey = await this.login();

    // Read the preview image from local storage
    const previewPath = await imageStorage.readPath(image.id, 'preview');
    const imageBuffer = await readFile(previewPath);
    const contentType = 'image/jpeg';

    // Submit image to Astrometry.net via file upload
    const form = new FormData();
    form.append('request-json', JSON.stringify({ session: sessionKey, apikey: this.astrometryApiKey }));
    form.append('file', new Blob([imageBuffer], { type: contentType }), image.filename || `image_${image.id}.jpg`);

    const uploadResponse = await fetch('https://nova.astrometry.net/api/upload', {
      method: 'POST',
      body: form,
      signal: AbortSignal.timeout(60000),
    });

    const uploadResult = await uploadResponse.json() as Record<string, unknown>;

    if (uploadResult.status !== "success") {
      throw new Error("Failed to submit image to Astrometry.net");
    }

    const submissionId = String(uploadResult.subid);

    // Create plate solving job record
    const job = await storage.createPlateSolvingJob({
      imageId: image.id,
      astrometrySubmissionId: submissionId,
      astrometryJobId: null,
      status: "processing",
      result: null,
    });

    // Emit real-time update via WebSocket
    if (wsManager) {
      wsManager.broadcast('plate-solving-update', {
        jobId: job.id,
        status: "processing",
        imageId: image.id,
        message: "Job submitted for plate solving"
      });
    }

    return { submissionId, jobId: job.id };
  }

  async pollForPlateSolvingResult(submissionId: string): Promise<PlateSolvingResult | null> {
    const astrometryConfig = await configService.getAstrometryConfig();
    const pollInterval = astrometryConfig.pollInterval * 1000;
    const maxAttempts = 720; // ~60 min at default 5s interval
    let attempts = 0;

    while (attempts < maxAttempts) {
      attempts++;
      try {
        const statusResponse = await fetch(
          `https://nova.astrometry.net/api/submissions/${submissionId}`,
          { headers: HEADERS }
        );
        const astrometryStatus = await statusResponse.json() as Record<string, unknown>;

        console.log(`Polling submission ${submissionId} - jobs:`, astrometryStatus.jobs, 'calibrations:', astrometryStatus.job_calibrations);

        if (astrometryStatus.job_calibrations && (astrometryStatus.job_calibrations as unknown[]).length > 0) {
          const jobId = (astrometryStatus.jobs as unknown[])?.[0]?.toString();
          if (!jobId) {
            throw new Error("No job ID found in successful submission");
          }
          return await this.fetchCompleteResult(jobId);
        } else if (astrometryStatus.jobs && (astrometryStatus.jobs as unknown[]).length > 0) {
          const jobStatus = (astrometryStatus.jobs as unknown[])[0];
          if (jobStatus === null) {
            return null;
          }
        }

        await new Promise(resolve => setTimeout(resolve, pollInterval));
      } catch (error: unknown) {
        const err = error as { cause?: { code?: string } };
        console.error("Error polling for plate solving result:", error);
        await new Promise(resolve => setTimeout(resolve, pollInterval));
      }
    }

    console.warn(`Plate solving timed out after ${maxAttempts} attempts for submission ${submissionId}`);
    return null;
  }

  async fetchCompleteResult(jobId: string): Promise<PlateSolvingResult> {
    // Get calibration details
    const calibrationResponse = await fetch(
      `https://nova.astrometry.net/api/jobs/${jobId}/calibration`,
      { headers: HEADERS }
    );
    const calibration: AstrometryCalibration = await calibrationResponse.json() as AstrometryCalibration;

    // Fetch annotations
    let annotations: AstrometryAnnotation[] = [];
    try {
      const annotationsResponse = await fetch(
        `https://nova.astrometry.net/api/jobs/${jobId}/annotations`,
        { headers: HEADERS }
      );
      let annotationsData = await annotationsResponse.json() as unknown;

      if (!Array.isArray(annotationsData)) {
        const arr = Object.values(annotationsData as Record<string, unknown>).find(v => Array.isArray(v));
        annotationsData = arr || [];
      }

      annotations = (annotationsData as Record<string, unknown>[]).map((annotation) => ({
        type: String(annotation.type || ''),
        names: Array.isArray(annotation.names) ? annotation.names as string[] : [],
        pixelx: annotation.pixelx != null ? Number(annotation.pixelx) : (annotation.pixel_x != null ? Number(annotation.pixel_x) : 0),
        pixely: annotation.pixely != null ? Number(annotation.pixely) : (annotation.pixel_y != null ? Number(annotation.pixel_y) : 0),
        ra: annotation.ra ? parseFloat(String(annotation.ra)) : 0,
        dec: annotation.dec ? parseFloat(String(annotation.dec)) : 0,
        radius: annotation.radius != null ? Number(annotation.radius) : undefined,
      }));
    } catch (error) {
      console.error(`Failed to fetch annotations for job ${jobId}:`, error);
    }

    // Fetch machine tags
    let machineTags: string[] = [];
    try {
      const tagsResponse = await fetch(
        `https://nova.astrometry.net/api/jobs/${jobId}/machine_tags/`,
        { headers: HEADERS }
      );
      const tagsData = await tagsResponse.json() as unknown;
      if (Array.isArray(tagsData)) {
        machineTags = tagsData;
      } else if (typeof tagsData === 'string') {
        machineTags = tagsData.split(',').map((t: string) => t.trim());
      } else if (tagsData && typeof tagsData === 'object' && Array.isArray((tagsData as Record<string, unknown>).tags)) {
        machineTags = (tagsData as Record<string, unknown>).tags as string[];
      }
    } catch (error) {
      console.error(`Failed to fetch machine tags for job ${jobId}:`, error);
    }

    return {
      calibration,
      annotations,
      machineTags: filterRelevantTags(machineTags)
    };
  }

  async updateJobAndImage(jobId: number, result: PlateSolvingResult): Promise<void> {
    const job = await storage.getPlateSolvingJob(jobId);
    if (!job) {
      throw new Error(`Job ${jobId} not found`);
    }

    await storage.updatePlateSolvingJob(job.id, {
      status: "success",
      result: {
        ...result.calibration,
        annotations: result.annotations as any
      } as any
    });

    if (job.imageId) {
      const image = await storage.getAstroImage(job.imageId);
      if (image) {
        const existingTags: string[] = image.tags || [];
        const relevantMachineTags = filterRelevantTags(result.machineTags);
        const allTags = Array.from(new Set([...existingTags, ...relevantMachineTags])).filter(Boolean);

        let constellation = null;
        if (result.calibration.ra && result.calibration.dec) {
          constellation = getConstellationFromCoordinates(result.calibration.ra, result.calibration.dec);
        }

        await storage.updateAstroImage(job.imageId, {
          plateSolved: true,
          ra: result.calibration.ra ? result.calibration.ra.toString() : null,
          dec: result.calibration.dec ? result.calibration.dec.toString() : null,
          pixelScale: result.calibration.pixscale || null,
          fieldOfView: result.calibration.radius ? `${(result.calibration.radius * 2).toFixed(1)}'` : null,
          rotation: result.calibration.orientation || null,
          astrometryJobId: job.astrometryJobId,
          constellation: constellation,
          tags: allTags as any,
        });

        try {
          const matches = await catalogService.matchTargetFromTags(allTags);
          if (matches.length > 0) {
            await storage.updateAstroImage(job.imageId, { targetName: matches[0].name } as any);
          }
        } catch (error) {
          console.error(`Failed to auto-match target for image ${image.id}:`, error);
        }

        try {
          const sidecarConfig = await configService.getSidecarConfig();
          if (sidecarConfig.enabled && job.astrometryJobId) {
            const equipment = await storage.getEquipmentForImage(job.imageId);
            await xmpSidecarService.writeSidecar(image, result, job.astrometryJobId, equipment, sidecarConfig);
          }
        } catch (error) {
          console.error(`Failed to write XMP sidecar for image ${image.id}:`, error);
        }

        if (wsManager) {
          wsManager.broadcast('plate-solving-update', {
            jobId: job.id,
            status: "success",
            imageId: job.imageId,
            result: {
              ...result.calibration,
              annotations: result.annotations
            }
          });
        }
      }
    }
  }

  async completePlateSolvingWorkflow(image: { id: number; immichId?: string | null; title?: string | null; filename?: string | null }): Promise<PlateSolvingResult> {
    const { submissionId, jobId } = await this.submitImageForPlateSolving(image);

    const result = await this.pollForPlateSolvingResult(submissionId);
    if (!result) {
      throw new Error("Plate solving failed");
    }

    await this.updateJobAndImage(jobId, result);

    return result;
  }

  async checkJobStatus(jobId: number): Promise<{ status: string; result?: PlateSolvingResult }> {
    const job = await storage.getPlateSolvingJob(jobId);
    if (!job) {
      throw new Error(`Job ${jobId} not found`);
    }

    try {
      const statusResponse = await fetch(
        `https://nova.astrometry.net/api/submissions/${job.astrometrySubmissionId}`,
        { headers: HEADERS }
      );
      const astrometryStatus = await statusResponse.json() as Record<string, unknown>;

      console.log(`Job ${job.id} status check - submission: ${job.astrometrySubmissionId}, jobs:`, astrometryStatus.jobs);

      if ((!job.astrometryJobId || job.astrometryJobId === "null") && astrometryStatus.jobs && (astrometryStatus.jobs as unknown[]).length > 0) {
        const newJobId = (astrometryStatus.jobs as unknown[])[0]?.toString();
        if (newJobId) {
          await storage.updatePlateSolvingJob(job.id, { astrometryJobId: newJobId });
          job.astrometryJobId = newJobId;
          console.log(`Updated job ${job.id} with astrometry job ID: ${newJobId}`);
        }
      }

      const submissionUrl = `https://nova.astrometry.net/status/${job.astrometrySubmissionId}`;
      const jobUrl = job.astrometryJobId ? `https://nova.astrometry.net/annotated_full/${job.astrometryJobId}` : null;

      const markFailed = async (error: string) => {
        const result = {
          error,
          submissionId: job.astrometrySubmissionId,
          astrometryJobId: job.astrometryJobId,
          submissionUrl,
          jobUrl,
        };
        await storage.updatePlateSolvingJob(job.id, { status: "failed", result });
        if (wsManager) {
          wsManager.broadcast('plate-solving-update', { jobId: job.id, status: "failed", result });
        }
        return { status: "failed" as const };
      };

      if (job.astrometryJobId) {
        try {
          const jobStatusResponse = await fetch(
            `https://nova.astrometry.net/api/jobs/${job.astrometryJobId}`,
            { headers: HEADERS }
          );

          if (jobStatusResponse.status === 404) {
            return markFailed(`Job not found on Astrometry.net. It may have expired (jobs expire after ~30 days).`);
          }

          const jobStatus = await jobStatusResponse.json() as Record<string, unknown>;
          if (jobStatus.status === "failure") {
            return markFailed(`Plate solving failed. Astrometry.net could not solve this image. This usually means the field of view or scale hints were incorrect, or the image quality was insufficient.`);
          }

          if (jobStatus.status === "success") {
            const result = await this.fetchCompleteResult(job.astrometryJobId);
            await this.updateJobAndImage(job.id, result);

            if (wsManager) {
              wsManager.broadcast('plate-solving-update', {
                jobId: job.id,
                status: "success",
                result: { ...result.calibration, annotations: result.annotations }
              });
            }

            return { status: "success", result };
          }
        } catch (jobError: unknown) {
          // If we can't check individual job status, continue with submission status check
        }
      }

      if (astrometryStatus.job_calibrations && (astrometryStatus.job_calibrations as unknown[]).length > 0) {
        const result = await this.fetchCompleteResult(job.astrometryJobId!);
        await this.updateJobAndImage(job.id, result);

        if (wsManager) {
          wsManager.broadcast('plate-solving-update', {
            jobId: job.id,
            status: "success",
            result: { ...result.calibration, annotations: result.annotations }
          });
        }

        return { status: "success", result };
      } else if (astrometryStatus.jobs && (astrometryStatus.jobs as unknown[]).length > 0) {
        const firstJob = (astrometryStatus.jobs as unknown[])[0];
        if (firstJob === null) {
          if (astrometryStatus.processing_finished) {
            return markFailed(`Plate solving failed. Astrometry.net could not solve this image.`);
          }
          return { status: "processing" };
        }
        return { status: "processing" };
      } else {
        return { status: "processing" };
      }
    } catch (error) {
      console.error("Error checking job status:", error);
      throw error;
    }
  }
}

export const astrometryService = new AstrometryService();
