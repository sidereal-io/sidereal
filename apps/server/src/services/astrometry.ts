import { storage } from './storage';
import { xmpSidecarService } from './xmp-sidecar';
import { configService } from './config';
import { getConstellationFromCoordinates } from './constellation-utils';
import { filterRelevantTags } from './tags-utils';
import { catalogService } from './catalog';
import { checkAstrometrySubmission, normalizeJobId, parseCalibration } from './astrometry-polling';

import type { WsManager } from './ws-manager';

let wsManager: WsManager | null = null;

export function setWsManager(wm: WsManager | null) {
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

export type PlateSolvingPollOutcome =
  | { status: 'success'; result: PlateSolvingResult; remoteJobId: string }
  | { status: 'failed'; error: string; remoteJobId: string | null }
  | { status: 'processing'; remoteJobId: string | null };
export type PlateSolvingWorkflowOutcome =
  | { status: 'success'; result: PlateSolvingResult }
  | { status: 'processing'; jobId: number; submissionId: string;
      message: 'Still processing. Check status later.' };

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

  async submitImageForPlateSolving(image: { id: number; fullUrl?: string | null; immichId?: string | null; title?: string | null; filename?: string | null }): Promise<{ submissionId: string; jobId: number }> {
    await this.ensureConfigLoaded();

    if (!image.fullUrl) {
      throw new Error("Image does not have a fullUrl");
    }

    if (!this.immichHost) {
      throw new Error("Immich host not configured");
    }

    if (!this.immichApiKey) {
      throw new Error("Immich API key not configured");
    }

    const sessionKey = await this.login();

    // Construct full URL to Immich server
    let fullImageUrl: string;
    if (image.fullUrl.startsWith('http')) {
      fullImageUrl = image.fullUrl;
    } else {
      const cleanHost = this.immichHost.endsWith('/') ? this.immichHost.slice(0, -1) : this.immichHost;
      const cleanPath = image.fullUrl.startsWith('/') ? image.fullUrl : `/${image.fullUrl}`;
      fullImageUrl = `${cleanHost}${cleanPath}`;
    }

    // Download image from Immich
    const imageResponse = await fetch(fullImageUrl, {
      headers: { 'X-API-Key': this.immichApiKey },
      signal: AbortSignal.timeout(30000),
    });
    if (!imageResponse.ok) {
      throw new Error(`Failed to download image from Immich: ${imageResponse.status}`);
    }
    const imageBuffer = Buffer.from(await imageResponse.arrayBuffer());
    const contentType = imageResponse.headers.get('content-type') || 'image/jpeg';

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

  async pollForPlateSolvingResult(
    submissionId: string,
    onJobId?: (remoteJobId: string) => Promise<void>,
  ): Promise<PlateSolvingPollOutcome> {
    const config = await configService.getAstrometryConfig();
    let selectedJobId: string | null = null;
    for (let attempt = 0; attempt < 720; attempt++) {
      try {
        const observation = await checkAstrometrySubmission(submissionId, selectedJobId);
        if (observation.jobId && observation.jobId !== selectedJobId) {
          await onJobId?.(observation.jobId);
          selectedJobId = observation.jobId;
        }
        if (observation.status === 'failed') return {
          status: 'failed', error: observation.error, remoteJobId: selectedJobId,
        };
        if (observation.status === 'success') return {
          status: 'success', result: await this.fetchCompleteResult(observation.jobId),
          remoteJobId: observation.jobId,
        };
      } catch (error) {
        console.error(`Error polling submission ${submissionId}:`, error);
      }
      await new Promise(resolve => setTimeout(resolve, config.pollInterval * 1000));
    }
    return { status: 'processing', remoteJobId: selectedJobId };
  }

  async fetchCompleteResult(jobId: string): Promise<PlateSolvingResult> {
    const normalizedId = normalizeJobId(jobId);
    if (!normalizedId) throw new Error('Invalid Astrometry job ID');
    jobId = normalizedId;
    // Get calibration details
    const calibrationResponse = await fetch(
      `https://nova.astrometry.net/api/jobs/${jobId}/calibration`,
      { headers: HEADERS }
    );
    if (!calibrationResponse.ok) throw new Error(`Calibration ${jobId}: HTTP ${calibrationResponse.status}`);
    const calibration = parseCalibration(await calibrationResponse.json());

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

  async completePlateSolvingWorkflow(image: { id: number; fullUrl?: string | null; immichId?: string | null; title?: string | null; filename?: string | null }): Promise<PlateSolvingWorkflowOutcome> {
    const { submissionId, jobId } = await this.submitImageForPlateSolving(image);
    const outcome = await this.pollForPlateSolvingResult(submissionId, async remoteJobId => {
      await storage.updatePlateSolvingJob(jobId, { astrometryJobId: remoteJobId });
    });
    if (outcome.status === 'failed') throw new Error(outcome.error);
    if (outcome.status === 'processing') return {
      status: 'processing', jobId, submissionId,
      message: 'Still processing. Check status later.',
    };
    await this.updateJobAndImage(jobId, outcome.result);
    return { status: 'success', result: outcome.result };
  }

  async checkJobStatus(jobId: number, options?: { resumeProcessing?: boolean }): Promise<{ status: string; result?: PlateSolvingResult }> {
    const job = await storage.getPlateSolvingJob(jobId);
    if (!job) throw new Error(`Job ${jobId} not found`);
    const submissionId = normalizeJobId(job.astrometrySubmissionId);
    if (!submissionId) throw new Error('Invalid Astrometry submission ID');
    const observation = await checkAstrometrySubmission(submissionId, normalizeJobId(job.astrometryJobId));
    const remoteJobId = normalizeJobId(observation.jobId);
    if (observation.status === 'processing') {
      const resume = options?.resumeProcessing === true && job.status === 'failed';
      if (remoteJobId !== null || resume) {
        await storage.updatePlateSolvingJob(job.id, {
          ...(remoteJobId !== null ? { astrometryJobId: remoteJobId } : {}),
          ...(resume ? { status: 'processing', result: null } : {}),
        });
      }
      return { status: 'processing' };
    }
    if (observation.status === 'success') {
      // Validate upstream results before changing any fields of a manual record.
      const result = await this.fetchCompleteResult(observation.jobId);
      await storage.updatePlateSolvingJob(job.id, { astrometryJobId: observation.jobId });
      await this.updateJobAndImage(job.id, result);
      if (wsManager) wsManager.broadcast('plate-solving-update', {
        jobId: job.id, status: 'success',
        result: { ...result.calibration, annotations: result.annotations },
      });
      return { status: 'success', result };
    }
    const result = {
      error: observation.error,
      submissionId,
      astrometryJobId: remoteJobId,
      submissionUrl: `https://nova.astrometry.net/status/${submissionId}`,
      jobUrl: remoteJobId ? `https://nova.astrometry.net/annotated_full/${remoteJobId}` : null,
    };
    await storage.updatePlateSolvingJob(job.id, {
      ...(remoteJobId !== null ? { astrometryJobId: remoteJobId } : {}),
      status: 'failed', result,
    });
    if (wsManager) wsManager.broadcast('plate-solving-update', { jobId: job.id, status: 'failed', result });
    return { status: 'failed' };
  }
}

export const astrometryService = new AstrometryService();
