/**
 * Seed script for local development.
 *
 * Populates the SQLite database with sample data so you can develop
 * against a realistic UI without needing a live Immich/Astrometry connection.
 *
 * Usage:
 *   npm run seed            # seed all tables
 *   npm run seed:reset      # clear all data, then seed
 */

import { db, schema } from '../../apps/server/src/db';
import { sql } from 'drizzle-orm';

const RESET = process.argv.includes('--reset');

// ---------------------------------------------------------------------------
// Seed data
// ---------------------------------------------------------------------------

const images = [
  {
    sourceType: 'immich',
    sourceId: 'seed-001',
    title: 'Orion Nebula (M42)',
    filename: 'orion_nebula.fits',
    thumbnailUrl: '/api/placeholder/400/300',
    fullUrl: '/api/placeholder/1200/900',
    captureDate: new Date('2024-12-15T22:30:00Z'),
    focalLength: 600,
    aperture: 'f/6.3',
    iso: 1600,
    exposureTime: '120s',
    frameCount: 60,
    totalIntegration: 2.0,
    telescope: 'Celestron EdgeHD 8"',
    camera: 'ZWO ASI294MC Pro',
    mount: 'Sky-Watcher EQ6-R Pro',
    filters: 'L-Pro',
    latitude: 34.0522,
    longitude: -118.2437,
    altitude: 350,
    plateSolved: true,
    ra: '83.822',
    dec: '-5.391',
    pixelScale: 1.23,
    fieldOfView: '1.2° x 0.9°',
    rotation: 45.2,
    astrometryJobId: 'astro-job-001',
    tags: ['nebula', 'emission', 'winter'],
    objectType: 'Nebula',
    constellation: 'Orion',
    description: 'The Great Orion Nebula, one of the brightest nebulae visible to the naked eye.',
  },
  {
    sourceType: 'immich',
    sourceId: 'seed-002',
    title: 'Andromeda Galaxy (M31)',
    filename: 'andromeda.fits',
    thumbnailUrl: '/api/placeholder/400/300',
    fullUrl: '/api/placeholder/1200/900',
    captureDate: new Date('2024-11-20T21:00:00Z'),
    focalLength: 250,
    aperture: 'f/4.9',
    iso: 800,
    exposureTime: '180s',
    frameCount: 45,
    totalIntegration: 2.25,
    telescope: 'William Optics RedCat 51',
    camera: 'Canon EOS Ra',
    mount: 'iOptron CEM26',
    filters: null,
    latitude: null,
    longitude: null,
    altitude: null,
    plateSolved: false,
    ra: null,
    dec: null,
    pixelScale: null,
    fieldOfView: null,
    rotation: null,
    astrometryJobId: null,
    tags: ['galaxy', 'spiral', 'autumn'],
    objectType: 'Galaxy',
    constellation: 'Andromeda',
    description: null,
  },
  {
    sourceType: 'immich',
    sourceId: 'seed-003',
    title: 'Ring Nebula (M57)',
    filename: 'ring_nebula.fits',
    thumbnailUrl: '/api/placeholder/400/300',
    fullUrl: '/api/placeholder/1200/900',
    captureDate: new Date('2024-10-05T23:15:00Z'),
    focalLength: 2000,
    aperture: 'f/10',
    iso: 3200,
    exposureTime: '60s',
    frameCount: 120,
    totalIntegration: 2.0,
    telescope: 'Celestron EdgeHD 8"',
    camera: 'ZWO ASI294MC Pro',
    mount: 'Sky-Watcher EQ6-R Pro',
    filters: 'OIII',
    latitude: 34.0522,
    longitude: -118.2437,
    altitude: 350,
    plateSolved: true,
    ra: '283.396',
    dec: '33.029',
    pixelScale: 0.39,
    fieldOfView: '0.3° x 0.2°',
    rotation: 12.8,
    astrometryJobId: 'astro-job-003',
    tags: ['nebula', 'planetary', 'summer'],
    objectType: 'Planetary Nebula',
    constellation: 'Lyra',
    description: 'The Ring Nebula, a well-known planetary nebula in the constellation Lyra.',
  },
  {
    sourceType: 'immich',
    sourceId: 'seed-004',
    title: 'Pleiades (M45)',
    filename: 'pleiades.fits',
    thumbnailUrl: '/api/placeholder/400/300',
    fullUrl: '/api/placeholder/1200/900',
    captureDate: new Date('2024-11-25T23:00:00Z'),
    focalLength: 400,
    aperture: 'f/5.6',
    iso: 1600,
    exposureTime: '90s',
    frameCount: 80,
    totalIntegration: 2.0,
    telescope: 'William Optics RedCat 51',
    camera: 'ZWO ASI294MC Pro',
    mount: 'iOptron CEM26',
    filters: null,
    latitude: 34.0522,
    longitude: -118.2437,
    altitude: 350,
    plateSolved: true,
    ra: '56.9',
    dec: '24.116',
    pixelScale: 2.4,
    fieldOfView: '2.0° x 2.0°',
    rotation: 0.0,
    astrometryJobId: 'astro-job-004',
    tags: ['star cluster', 'open cluster', 'winter'],
    objectType: 'Open Cluster',
    constellation: 'Taurus',
    description: 'The Pleiades, also known as the Seven Sisters and Messier 45, is an open star cluster containing middle-aged, hot B-type stars in the north-west of the constellation Taurus.',
  },
];

const equipmentList = [
  {
    name: 'Celestron EdgeHD 8"',
    type: 'telescope',
    specifications: ({ focalLength: '2032mm', aperture: '203mm', fRatio: 'f/10' }),
    description: 'Schmidt-Cassegrain telescope with EdgeHD optics for flat-field imaging',
  },
  {
    name: 'ZWO ASI294MC Pro',
    type: 'camera',
    specifications: ({ sensor: 'IMX294', resolution: '4144x2822', pixelSize: '4.63µm' }),
    description: 'Cooled color CMOS camera for deep sky imaging',
  },
  {
    name: 'Sky-Watcher EQ6-R Pro',
    type: 'mount',
    specifications: ({ payload: '20kg', tracking: 'GoTo', type: 'German Equatorial' }),
    description: 'Computerized equatorial mount with belt-driven motors',
  },
  {
    name: 'Optolong L-Pro',
    type: 'filter',
    specifications: ({ type: 'Light pollution', bandwidth: 'Multi-bandpass' }),
    description: 'Light pollution filter for broadband imaging from urban areas',
  },
  {
    name: 'William Optics RedCat 51',
    type: 'telescope',
    specifications: ({ focalLength: '250mm', aperture: '51mm', fRatio: 'f/4.9' }),
    description: 'Compact Petzval astrograph for wide-field astrophotography',
  },
  {
    name: 'Canon EOS Ra',
    type: 'camera',
    specifications: ({ sensor: 'Full Frame CMOS', resolution: '6720x4480', pixelSize: '5.36µm' }),
    description: 'Full-frame mirrorless camera modified for astrophotography with enhanced H-alpha sensitivity',
  },
];

const notificationsList = [
  {
    type: 'error',
    title: 'Plate solving failed',
    message: 'Failed to plate solve image orion_nebula.fits: timeout exceeded',
    details: ({ imageId: 1, jobId: 'astro-job-fail-001' }),
    acknowledged: false,
    createdAt: new Date('2024-12-16T10:30:00Z'),
  },
  {
    type: 'success',
    title: 'Sync completed',
    message: 'Successfully synced 12 new images from Immich',
    details: ({ newImages: 12, updatedImages: 3 }),
    acknowledged: false,
    createdAt: new Date('2024-12-16T09:00:00Z'),
  },
  {
    type: 'warning',
    title: 'Astrometry rate limit',
    message: 'Approaching Astrometry.net API rate limit. 8 of 10 requests used.',
    details: null,
    acknowledged: false,
    createdAt: new Date('2024-12-16T08:45:00Z'),
  },
  {
    type: 'info',
    title: 'Worker restarted',
    message: 'Background worker process was restarted automatically',
    details: ({ restartCount: 2 }),
    acknowledged: false,
    createdAt: new Date('2024-12-16T07:30:00Z'),
  },
  {
    type: 'error',
    title: 'Immich connection lost',
    message: 'Unable to connect to Immich server at http://localhost:2283',
    details: ({ host: 'http://localhost:2283' }),
    acknowledged: false,
    createdAt: new Date('2024-12-16T06:00:00Z'),
  },
  {
    type: 'success',
    title: 'Plate solving completed',
    message: 'Successfully plate solved ring_nebula.fits',
    details: ({ imageId: 3, ra: '18h 53m 35.1s', dec: "+33° 01' 45\"" }),
    acknowledged: false,
    createdAt: new Date('2024-12-15T23:00:00Z'),
  },
  {
    type: 'warning',
    title: 'Low disk space',
    message: 'Server disk usage is at 85%. Consider cleaning up old files.',
    details: ({ diskUsage: 85 }),
    acknowledged: false,
    createdAt: new Date('2024-12-15T20:00:00Z'),
  },
  {
    type: 'info',
    title: 'Scheduled sync started',
    message: 'Automatic Immich sync job started',
    details: null,
    acknowledged: false,
    createdAt: new Date('2024-12-15T16:00:00Z'),
  },
];

const plateSolvingJobs = [
  {
    imageId: 1,
    astrometrySubmissionId: 'sub-001',
    astrometryJobId: 'astro-job-001',
    status: 'success',
    submittedAt: new Date('2024-12-15T22:45:00Z'),
    completedAt: new Date('2024-12-15T23:00:00Z'),
    result: ({ ra: 83.82, dec: -5.39, pixelScale: 1.23, orientation: 45.2 }),
  },
  {
    imageId: 2,
    astrometrySubmissionId: 'sub-002',
    astrometryJobId: null,
    status: 'pending',
    submittedAt: new Date('2024-12-16T10:00:00Z'),
    completedAt: null,
    result: null,
  },
  {
    imageId: 3,
    astrometrySubmissionId: 'sub-003',
    astrometryJobId: 'astro-job-003',
    status: 'success',
    submittedAt: new Date('2024-10-05T23:30:00Z'),
    completedAt: new Date('2024-10-05T23:50:00Z'),
    result: ({ ra: 283.39, dec: 33.03, pixelScale: 0.39, orientation: 12.8 }),
  },
  {
    imageId: 1,
    astrometrySubmissionId: 'sub-004',
    astrometryJobId: 'astro-job-fail-001',
    status: 'failed',
    submittedAt: new Date('2024-12-16T10:15:00Z'),
    completedAt: new Date('2024-12-16T10:30:00Z'),
    result: ({ error: 'Timeout exceeded' }),
  },
  {
    imageId: 4,
    astrometrySubmissionId: 'sub-005',
    astrometryJobId: 'astro-job-004',
    status: 'success',
    submittedAt: new Date('2024-11-25T23:15:00Z'),
    completedAt: new Date('2024-11-25T23:30:00Z'),
    result: ({ ra: 56.85, dec: 24.1166, pixelScale: 2.4, orientation: 0.0 }),
  },
];

// Image-equipment mappings: [imageId, equipmentName]
const imageEquipmentMap = [
  [1, 'Celestron EdgeHD 8"'],
  [1, 'ZWO ASI294MC Pro'],
  [1, 'Sky-Watcher EQ6-R Pro'],
  [1, 'Optolong L-Pro'],
  [2, 'William Optics RedCat 51'],
  [2, 'Canon EOS Ra'],
  [3, 'Celestron EdgeHD 8"'],
  [3, 'ZWO ASI294MC Pro'],
  [3, 'Sky-Watcher EQ6-R Pro'],
  [4, 'William Optics RedCat 51'],
  [4, 'ZWO ASI294MC Pro'],
] as const;

// ---------------------------------------------------------------------------
// Seed logic
// ---------------------------------------------------------------------------

async function clearAll() {
  console.log('Clearing existing data...');
  await db.delete(schema.imageEquipment).execute();
  await db.delete(schema.plateSolvingJobs).execute();
  await db.delete(schema.notifications).execute();
  await db.delete(schema.equipment).execute();
  await db.delete(schema.astrophotographyImages).execute();
  console.log('  Done.');
}

async function seedImages() {
  const existing = await db.select({ id: schema.astrophotographyImages.id }).from(schema.astrophotographyImages).execute();
  if (existing.length > 0) {
    console.log(`  Skipping images (${existing.length} already exist)`);
    return;
  }

  for (const img of images) {
    await db.insert(schema.astrophotographyImages).values(img).execute();
  }
  console.log(`  Inserted ${images.length} images`);
}

async function seedEquipment() {
  const existing = await db.select({ id: schema.equipment.id }).from(schema.equipment).execute();
  if (existing.length > 0) {
    console.log(`  Skipping equipment (${existing.length} already exist)`);
    return;
  }

  for (const eq of equipmentList) {
    await db.insert(schema.equipment).values(eq).execute();
  }
  console.log(`  Inserted ${equipmentList.length} equipment items`);
}

async function seedImageEquipment() {
  const existing = await db.select({ id: schema.imageEquipment.id }).from(schema.imageEquipment).execute();
  if (existing.length > 0) {
    console.log(`  Skipping image-equipment links (${existing.length} already exist)`);
    return;
  }

  // Build name→id map from inserted equipment
  const allEquipment = await db.select().from(schema.equipment).execute();
  const nameToId = new Map(allEquipment.map((e: any) => [e.name, e.id]));

  for (const [imageId, equipName] of imageEquipmentMap) {
    const equipmentId = nameToId.get(equipName);
    if (equipmentId) {
      await db.insert(schema.imageEquipment).values({ imageId, equipmentId }).execute();
    }
  }
  console.log(`  Linked ${imageEquipmentMap.length} image-equipment pairs`);
}

async function seedNotifications() {
  const existing = await db.select({ id: schema.notifications.id }).from(schema.notifications).execute();
  if (existing.length > 0) {
    console.log(`  Skipping notifications (${existing.length} already exist)`);
    return;
  }

  for (const n of notificationsList) {
    await db.insert(schema.notifications).values(n).execute();
  }
  console.log(`  Inserted ${notificationsList.length} notifications`);
}

async function seedPlateSolvingJobs() {
  const existing = await db.select({ id: schema.plateSolvingJobs.id }).from(schema.plateSolvingJobs).execute();
  if (existing.length > 0) {
    console.log(`  Skipping plate solving jobs (${existing.length} already exist)`);
    return;
  }

  for (const job of plateSolvingJobs) {
    await db.insert(schema.plateSolvingJobs).values(job).execute();
  }
  console.log(`  Inserted ${plateSolvingJobs.length} plate solving jobs`);
}

async function main() {
  console.log(RESET ? 'Resetting and seeding database...' : 'Seeding database...');
  console.log();

  if (RESET) {
    await clearAll();
    console.log();
  }

  await seedImages();
  await seedEquipment();
  await seedImageEquipment();
  await seedNotifications();
  await seedPlateSolvingJobs();

  console.log();
  console.log('Seed complete!');
  process.exit(0);
}

main().catch((err) => {
  console.error('Seed failed:', err);
  process.exit(1);
});
