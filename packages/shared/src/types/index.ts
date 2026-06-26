// Re-export database types
export type { AstroImage, InsertAstroImage, Equipment, InsertEquipment, ImageEquipment, InsertImageEquipment, PlateSolvingJob, InsertPlateSolvingJob, EquipmentGroup, InsertEquipmentGroup, EquipmentGroupMember, InsertEquipmentGroupMember, Location, InsertLocation, ImageAcquisitionRow, InsertImageAcquisitionRow, CatalogObject, InsertCatalogObject, UserTarget, InsertUserTarget } from '../db/pg-schema';

// Notification types
export type NotificationType = 'error' | 'warning' | 'info' | 'success';

export interface Notification {
  id: number;
  type: NotificationType;
  title: string;
  message: string;
  details?: Record<string, unknown>;
  createdAt: string;
  acknowledged: boolean;
}

export type InsertNotification = Pick<Notification, 'type' | 'title' | 'message'> & {
  details?: Record<string, unknown>;
};

// Equipment type string literal
export type EquipmentType = 'telescope' | 'camera' | 'mount' | 'filter' | 'accessory' | 'software';

// Equipment specification interfaces - known fields per equipment type
// All data serializes into the `specifications` JSON column

export interface TelescopeSpecifications {
  focalLength?: number;       // mm
  apertureDiameter?: number;  // mm
  focalRatio?: string;        // e.g. "f/5"
  design?: 'refractor' | 'reflector' | 'SCT' | 'Cassegrain' | 'other';
  [key: string]: unknown;
}

export interface CameraSpecifications {
  sensorType?: 'CMOS' | 'CCD';
  sensorSize?: string;        // e.g. "APS-C", "Full Frame", "4/3"
  pixelSize?: number;         // microns
  resolution?: string;        // e.g. "6200x4152"
  cooled?: boolean;
  [key: string]: unknown;
}

export interface MountSpecifications {
  mountType?: 'EQ' | 'Alt-Az' | 'Star Tracker';
  payloadCapacity?: number;   // kg
  gotoCapable?: boolean;
  [key: string]: unknown;
}

export interface FilterSpecifications {
  filterType?: 'Narrowband' | 'Broadband' | 'UV-IR Cut' | 'Light Pollution';
  bandwidth?: number;         // nm
  wavelength?: number;        // nm center
  [key: string]: unknown;
}

export interface AccessorySpecifications {
  [key: string]: unknown;
}

export type EquipmentSpecifications =
  | TelescopeSpecifications
  | CameraSpecifications
  | MountSpecifications
  | FilterSpecifications
  | AccessorySpecifications;

// Known field definitions for UI rendering
export interface SpecFieldDefinition {
  key: string;
  label: string;
  type: 'number' | 'text' | 'boolean' | 'select';
  unit?: string;
  placeholder?: string;
  options?: string[];
}

export const EQUIPMENT_SPEC_FIELDS: Record<EquipmentType, SpecFieldDefinition[]> = {
  telescope: [
    { key: 'focalLength', label: 'Focal Length', type: 'number', unit: 'mm' },
    { key: 'apertureDiameter', label: 'Aperture Diameter', type: 'number', unit: 'mm' },
    { key: 'focalRatio', label: 'Focal Ratio', type: 'text' },
    { key: 'design', label: 'Design', type: 'select', options: ['refractor', 'reflector', 'SCT', 'Cassegrain', 'other'] },
  ],
  camera: [
    { key: 'sensorType', label: 'Sensor Type', type: 'select', options: ['CMOS', 'CCD'] },
    { key: 'sensorSize', label: 'Sensor Size', type: 'text' },
    { key: 'pixelSize', label: 'Pixel Size', type: 'number', unit: 'µm' },
    { key: 'resolution', label: 'Resolution', type: 'text', unit: 'px', placeholder: 'e.g. 6200x4152' },
    { key: 'cooled', label: 'Cooled', type: 'boolean' },
  ],
  mount: [
    { key: 'mountType', label: 'Mount Type', type: 'select', options: ['EQ', 'Alt-Az', 'Star Tracker'] },
    { key: 'payloadCapacity', label: 'Payload Capacity', type: 'number', unit: 'kg' },
    { key: 'gotoCapable', label: 'GoTo Capable', type: 'boolean' },
  ],
  filter: [
    { key: 'filterType', label: 'Filter Type', type: 'select', options: ['Narrowband', 'Broadband', 'UV-IR Cut', 'Light Pollution'] },
    { key: 'bandwidth', label: 'Bandwidth', type: 'number', unit: 'nm' },
    { key: 'wavelength', label: 'Wavelength', type: 'number', unit: 'nm' },
  ],
  accessory: [],
  software: [],
};

// Image source plugin types
export type {
  ImageSourcePlugin,
  SourceMetadata,
  IngestResult,
  ConnectionResult,
  SourceStatus,
} from './image-source.js';
