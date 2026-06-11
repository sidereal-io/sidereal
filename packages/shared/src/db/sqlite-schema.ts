// IMPORTANT: This schema must stay in sync with pg-schema.ts.
// When adding/removing columns, update BOTH files.
import { sqliteTable, text, integer, real } from 'drizzle-orm/sqlite-core';

export const astrophotographyImages = sqliteTable('astrophotography_images', {
    id: integer('id').primaryKey(),
    immichId: text('immich_id').unique(),
    sourceType: text('source_type').notNull().default('immich'),
    sourceId: text('source_id'),
    title: text('title').notNull(),
    filename: text('filename').notNull(),
    originalPath: text('original_path'),
    captureDate: integer('capture_date', { mode: 'timestamp' }),
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
    plateSolved: integer('plate_solved', { mode: 'boolean' }).default(false),
    ra: text('ra'),
    dec: text('dec'),
    pixelScale: real('pixel_scale'),
    fieldOfView: text('field_of_view'),
    rotation: real('rotation'),
    astrometryJobId: text('astrometry_job_id'),
    tags: text('tags', { mode: 'json' }),
    objectType: text('object_type'),
    constellation: text('constellation'),
    targetName: text('target_name'),
    description: text('description'),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
    updatedAt: integer('updated_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const equipment = sqliteTable('equipment', {
    id: integer('id').primaryKey(),
    name: text('name').notNull(),
    type: text('type').notNull(),
    specifications: text('specifications', { mode: 'json' }),
    imageUrl: text('image_url'),
    description: text('description'),
    cost: real('cost'),
    acquisitionDate: integer('acquisition_date', { mode: 'timestamp' }),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
    updatedAt: integer('updated_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const imageEquipment = sqliteTable('image_equipment', {
    id: integer('id').primaryKey(),
    imageId: integer('image_id').references(() => astrophotographyImages.id, { onDelete: 'cascade' }),
    equipmentId: integer('equipment_id').references(() => equipment.id, { onDelete: 'cascade' }),
    settings: text('settings', { mode: 'json' }),
    notes: text('notes'),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const plateSolvingJobs = sqliteTable('plate_solving_jobs', {
    id: integer('id').primaryKey(),
    imageId: integer('image_id').references(() => astrophotographyImages.id),
    astrometrySubmissionId: text('astrometry_submission_id'),
    astrometryJobId: text('astrometry_job_id'),
    status: text('status').notNull().default('pending'),
    submittedAt: integer('submitted_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
    completedAt: integer('completed_at', { mode: 'timestamp' }),
    result: text('result', { mode: 'json' }),
});

export const adminSettings = sqliteTable('admin_settings', {
    id: integer('id').primaryKey(),
    key: text('key').unique().notNull(),
    value: text('value', { mode: 'json' }),
});

export const notifications = sqliteTable('notifications', {
    id: integer('id').primaryKey(),
    type: text('type').notNull(), // 'error' | 'warning' | 'info' | 'success'
    title: text('title').notNull(),
    message: text('message').notNull(),
    details: text('details', { mode: 'json' }),
    acknowledged: integer('acknowledged', { mode: 'boolean' }).default(false),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const imageAcquisition = sqliteTable('image_acquisition', {
    id: integer('id').primaryKey(),
    imageId: integer('image_id').notNull().references(() => astrophotographyImages.id, { onDelete: 'cascade' }),
    filterId: integer('filter_id').references(() => equipment.id, { onDelete: 'set null' }),
    filterName: text('filter_name'),
    frameCount: integer('frame_count').notNull(),
    exposureTime: real('exposure_time').notNull(),
    gain: integer('gain'),
    offset: integer('offset'),
    binning: text('binning'),
    sensorTemp: real('sensor_temp'),
    date: integer('date', { mode: 'timestamp' }),
    notes: text('notes'),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const equipmentGroups = sqliteTable('equipment_groups', {
    id: integer('id').primaryKey(),
    name: text('name').notNull(),
    description: text('description'),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
    updatedAt: integer('updated_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const equipmentGroupMembers = sqliteTable('equipment_group_members', {
    id: integer('id').primaryKey(),
    groupId: integer('group_id').notNull().references(() => equipmentGroups.id, { onDelete: 'cascade' }),
    equipmentId: integer('equipment_id').notNull().references(() => equipment.id, { onDelete: 'cascade' }),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const catalogObjects = sqliteTable('catalog_objects', {
    id: integer('id').primaryKey(),
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
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const locations = sqliteTable('locations', {
    id: integer('id').primaryKey(),
    name: text('name').notNull(),
    latitude: real('latitude').notNull(),
    longitude: real('longitude').notNull(),
    altitude: real('altitude'),
    description: text('description'),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
    updatedAt: integer('updated_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});

export const userTargets = sqliteTable('user_targets', {
    id: integer('id').primaryKey(),
    catalogName: text('catalog_name').notNull().unique(),
    notes: text('notes'),
    tags: text('tags', { mode: 'json' }),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
    updatedAt: integer('updated_at', { mode: 'timestamp' }).notNull().$defaultFn(() => new Date()),
});