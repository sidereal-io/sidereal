export interface SourceMetadata {
  filename: string;
  captureDate?: Date | null;
  focalLength?: number | null;
  aperture?: string | null;
  iso?: number | null;
  exposureTime?: string | null;
  latitude?: number | null;
  longitude?: number | null;
  altitude?: number | null;
  camera?: string | null;
  description?: string | null;
}

export interface IngestResult {
  imageId: number;
  filename: string;
  sourceType: string;
  sourceId: string;
}

export interface ConnectionResult {
  ok: boolean;
  message?: string;
}

export interface SourceStatus {
  sourceType: string;
  lastSync: Date | null;
  imageCount: number;
  healthy: boolean;
  message?: string;
}

export interface ImageSourcePlugin {
  readonly sourceType: string;
  readonly displayName: string;

  testConnection(): Promise<ConnectionResult>;
  getStatus(): Promise<SourceStatus>;
}
