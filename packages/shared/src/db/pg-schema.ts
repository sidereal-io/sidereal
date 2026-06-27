// IMPORTANT: This schema must stay in sync with sqlite-schema.ts.
// When adding/removing columns, update BOTH files.
import { pgTable, serial, text, timestamp, real, integer, boolean, json } from 'drizzle-orm/pg-core';
import { createInsertSchema } from 'drizzle-zod';
import { z } from 'zod';

export const astrophotographyImages = pgTable('astrophotography_images', {
  id: serial('id').primaryKey(),
  sourceType: text('source_type').notNull().default('immich'),
  sourceId: text('source_id'),
  title: text('title').notNull(),
  filename: text('filename').notNull(),
  originalPath: text('original_path'),
  captureDate: timestamp('capture_date'),
  focalLength: real('focal_length'),
  aperture: text('aperture'),
  iso: integer('iso'),
  exposureTime: text('exposure_time'),
  frameCount: integer('frame_count'),
  totalIntegration: real('total_integration_hours'),
  telescope: text('telescope'),
  camera: text('camera'),
  mount: text('mount'),
  filters: text('filters'),
  latitude: real('latitude'),
  longitude: real('longitude'),
  altitude: real('altitude'),
  plateSolved: boolean('plate_solved').default(false),
  ra: text('ra'),
  dec: text('dec'),
  pixelScale: real('pixel_scale'),
  fieldOfView: text('field_of_view'),
  rotation: real('rotation'),
  astrometryJobId: text('astrometry_job_id'),
  tags: text('tags').array(),
  objectType: text('object_type'),
  constellation: text('constellation'),
  targetName: text('target_name'),
  description: text('description'),
  createdAt: timestamp('created_at').defaultNow(),
  updatedAt: timestamp('updated_at').defaultNow(),
});

export const equipment = pgTable('equipment', {
  id: serial('id').primaryKey(),
  name: text('name').notNull(),
  type: text('type').notNull(),
  specifications: json('specifications'),
  imageUrl: text('image_url'),
  description: text('description'),
  cost: real('cost'),
  acquisitionDate: timestamp('acquisition_date'),
  createdAt: timestamp('created_at').defaultNow(),
  updatedAt: timestamp('updated_at').defaultNow(),
});

export const imageEquipment = pgTable('image_equipment', {
    id: serial('id').primaryKey(),
    imageId: integer('image_id').references(() => astrophotographyImages.id, { onDelete: 'cascade' }),
    equipmentId: integer('equipment_id').references(() => equipment.id, { onDelete: 'cascade' }),
    settings: json('settings'),
    notes: text('notes'),
    createdAt: timestamp('created_at').defaultNow(),
});

export const plateSolvingJobs = pgTable('plate_solving_jobs', {
    id: serial('id').primaryKey(),
    imageId: integer('image_id').references(() => astrophotographyImages.id),
    astrometrySubmissionId: text('astrometry_submission_id'),
    astrometryJobId: text('astrometry_job_id'),
    status: text('status').notNull().default('pending'),
    submittedAt: timestamp('submitted_at').defaultNow(),
    completedAt: timestamp('completed_at'),
    result: json('result'),
});

export const adminSettings = pgTable('admin_settings', {
    id: serial('id').primaryKey(),
    key: text('key').unique().notNull(),
    value: json('value'),
});

export const notifications = pgTable('notifications', {
    id: serial('id').primaryKey(),
    type: text('type').notNull(), // 'error' | 'warning' | 'info' | 'success'
    title: text('title').notNull(),
    message: text('message').notNull(),
    details: json('details'),
    acknowledged: boolean('acknowledged').default(false),
    createdAt: timestamp('created_at').defaultNow(),
});

export const imageAcquisition = pgTable('image_acquisition', {
    id: serial('id').primaryKey(),
    imageId: integer('image_id').notNull().references(() => astrophotographyImages.id, { onDelete: 'cascade' }),
    filterId: integer('filter_id').references(() => equipment.id, { onDelete: 'set null' }),
    filterName: text('filter_name'),
    frameCount: integer('frame_count').notNull(),
    exposureTime: real('exposure_time').notNull(),
    gain: integer('gain'),
    offset: integer('offset'),
    binning: text('binning'),
    sensorTemp: real('sensor_temp'),
    date: timestamp('date'),
    notes: text('notes'),
    createdAt: timestamp('created_at').defaultNow(),
});

export const equipmentGroups = pgTable('equipment_groups', {
  id: serial('id').primaryKey(),
  name: text('name').notNull(),
  description: text('description'),
  createdAt: timestamp('created_at').defaultNow(),
  updatedAt: timestamp('updated_at').defaultNow(),
});

export const equipmentGroupMembers = pgTable('equipment_group_members', {
  id: serial('id').primaryKey(),
  groupId: integer('group_id').notNull().references(() => equipmentGroups.id, { onDelete: 'cascade' }),
  equipmentId: integer('equipment_id').notNull().references(() => equipment.id, { onDelete: 'cascade' }),
  createdAt: timestamp('created_at').defaultNow(),
});

export const catalogObjects = pgTable('catalog_objects', {
  id: serial('id').primaryKey(),
  name: text('name').notNull().unique(),
  type: text('type'),
  ra: text('ra'),
  dec: text('dec'),
  raDeg: real('ra_deg'),
  decDeg: real('dec_deg'),
  constellation: text('constellation'),
  majorAxis: real('major_axis'),
  minorAxis: real('minor_axis'),
  bMag: real('b_mag'),
  vMag: real('v_mag'),
  surfaceBrightness: real('surface_brightness'),
  hubbleType: text('hubble_type'),
  messier: text('messier'),
  ngcRef: text('ngc_ref'),
  icRef: text('ic_ref'),
  commonNames: text('common_names'),
  identifiers: text('identifiers'),
  createdAt: timestamp('created_at').defaultNow(),
});

export const locations = pgTable('locations', {
  id: serial('id').primaryKey(),
  name: text('name').notNull(),
  latitude: real('latitude').notNull(),
  longitude: real('longitude').notNull(),
  altitude: real('altitude'),
  description: text('description'),
  createdAt: timestamp('created_at').defaultNow(),
  updatedAt: timestamp('updated_at').defaultNow(),
});

export const userTargets = pgTable('user_targets', {
  id: serial('id').primaryKey(),
  catalogName: text('catalog_name').notNull().unique(),
  notes: text('notes'),
  tags: json('tags').$type<string[]>(),
  createdAt: timestamp('created_at').defaultNow(),
  updatedAt: timestamp('updated_at').defaultNow(),
});


// Zod schemas for validation
export const insertAstroImageSchema = createInsertSchema(astrophotographyImages);
export const insertEquipmentSchema = createInsertSchema(equipment);
export const insertImageEquipmentSchema = createInsertSchema(imageEquipment);
export const insertPlateSolvingJobSchema = createInsertSchema(plateSolvingJobs);
export const insertImageAcquisitionSchema = createInsertSchema(imageAcquisition);
export const insertEquipmentGroupSchema = createInsertSchema(equipmentGroups);
export const insertEquipmentGroupMemberSchema = createInsertSchema(equipmentGroupMembers);
export const insertLocationSchema = createInsertSchema(locations);
export const insertCatalogObjectSchema = createInsertSchema(catalogObjects);
export const insertUserTargetSchema = createInsertSchema(userTargets);

export type AstroImage = typeof astrophotographyImages.$inferSelect;
export type InsertAstroImage = z.infer<typeof insertAstroImageSchema>;
export type Equipment = typeof equipment.$inferSelect;
export type InsertEquipment = z.infer<typeof insertEquipmentSchema>;
export type ImageEquipment = typeof imageEquipment.$inferSelect;
export type InsertImageEquipment = z.infer<typeof insertImageEquipmentSchema>;
export type PlateSolvingJob = typeof plateSolvingJobs.$inferSelect;
export type InsertPlateSolvingJob = z.infer<typeof insertPlateSolvingJobSchema>;
export type ImageAcquisitionRow = typeof imageAcquisition.$inferSelect;
export type InsertImageAcquisitionRow = z.infer<typeof insertImageAcquisitionSchema>;
export type EquipmentGroup = typeof equipmentGroups.$inferSelect;
export type InsertEquipmentGroup = z.infer<typeof insertEquipmentGroupSchema>;
export type EquipmentGroupMember = typeof equipmentGroupMembers.$inferSelect;
export type InsertEquipmentGroupMember = z.infer<typeof insertEquipmentGroupMemberSchema>;
export type Location = typeof locations.$inferSelect;
export type InsertLocation = z.infer<typeof insertLocationSchema>;
export type CatalogObject = typeof catalogObjects.$inferSelect;
export type InsertCatalogObject = z.infer<typeof insertCatalogObjectSchema>;
export type UserTarget = typeof userTargets.$inferSelect;
export type InsertUserTarget = z.infer<typeof insertUserTargetSchema>;
