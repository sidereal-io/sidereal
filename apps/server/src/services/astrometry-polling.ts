import { z } from 'zod';

const submissionSchema = z.object({
  jobs: z.array(z.unknown()).nullable().optional(),
  job_calibrations: z.array(z.unknown()).nullable().optional(),
  error_message: z.string().nullable().optional(),
  status: z.string().optional(),
}).passthrough();
const jobSchema = z.object({ status: z.string().optional() }).passthrough();
const calibrationSchema = z.object({
  ra: z.number().finite(), dec: z.number().finite(),
  pixscale: z.number().finite(), radius: z.number().finite(),
  orientation: z.number().finite(), width_arcsec: z.number().finite().optional(),
  height_arcsec: z.number().finite().optional(), parity: z.number().finite().optional(),
});

export function normalizeJobId(value: unknown): string | null {
  if (typeof value !== 'number' &&
      !(typeof value === 'string' && /^\d+$/.test(value))) return null;
  const id = Number(value);
  return Number.isSafeInteger(id) && id > 0 ? String(id) : null;
}

export const parseCalibration = (value: unknown) => calibrationSchema.parse(value);

export type PollObservation =
  | { status: 'processing'; jobId: string | null }
  | { status: 'success'; jobId: string }
  | { status: 'failed'; jobId: string | null; error: string };

export async function checkAstrometrySubmission(
  submissionId: string, selectedJobId: string | null, fetchFn: typeof fetch = fetch,
): Promise<PollObservation> {
  const headers = { 'User-Agent': 'Mozilla/5.0 (compatible; Sidereal/1.0)' };
  const response = await fetchFn(`https://nova.astrometry.net/api/submissions/${submissionId}`, { headers });
  if (!response.ok) throw new Error(`Submission ${submissionId}: HTTP ${response.status}`);
  const submission = await parseResponse(response, submissionSchema, `Submission ${submissionId}`);
  if (submission.status === 'error') throw new Error(`Submission ${submissionId}: API request error`);
  let calibratedJob: string | null = null;
  for (const pair of submission.job_calibrations ?? []) {
    if (!Array.isArray(pair) || pair.length !== 2) continue;
    const id = normalizeJobId(pair[0]);
    if (id && normalizeJobId(pair[1])) { calibratedJob = id; break; }
  }
  if (calibratedJob) return { status: 'success', jobId: calibratedJob };
  const jobId = normalizeJobId(selectedJobId)
    ?? (submission.jobs ?? []).map(normalizeJobId).find(id => id !== null) ?? null;
  if (submission.error_message?.trim()) {
    return { status: 'failed', jobId, error: submission.error_message };
  }
  if (!jobId) return { status: 'processing', jobId: null };
  const jobResponse = await fetchFn(`https://nova.astrometry.net/api/jobs/${jobId}`, { headers });
  if (jobResponse.status === 404) return {
    status: 'failed', jobId, error: 'Job not found on Astrometry.net. It may be unavailable.',
  };
  if (!jobResponse.ok) throw new Error(`Job ${jobId}: HTTP ${jobResponse.status}`);
  const job = await parseResponse(jobResponse, jobSchema, `Job ${jobId}`);
  if (job.status === 'error') throw new Error(`Job ${jobId}: API request error`);
  if (job.status === 'success') return { status: 'success', jobId };
  if (job.status === 'failure') return {
    status: 'failed', jobId, error: 'Plate solving failed. Astrometry.net could not solve this image.',
  };
  return { status: 'processing', jobId };
}

async function parseResponse<T>(response: Response, schema: z.ZodType<T>, context: string): Promise<T> {
  try {
    return schema.parse(await response.json());
  } catch (cause) {
    throw new Error(`${context}: Invalid API response`, { cause });
  }
}
