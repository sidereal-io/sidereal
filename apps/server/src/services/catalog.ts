import { db, schema } from '../db';
import { eq, like, or, sql, and, lte, gte, isNotNull, inArray, asc, desc, SQL } from 'drizzle-orm';
import { storage } from './storage';
import type { InsertCatalogObject, CatalogObject } from '@shared/types';
import { normalizeObjectName, raToDecimalDegrees, decToDecimalDegrees, parseNGCCSV, sortCatalogMatchesByPriority } from '@shared/catalog-utils';

// Re-export for consumers that imported from here
export { normalizeObjectName, raToDecimalDegrees, decToDecimalDegrees };

/**
 * Build a case-insensitive OR condition across catalog search columns.
 * Searches both the raw query and the normalized form (e.g. "m45" → "M 45").
 * Uses LOWER() for cross-DB compatibility (SQLite + PG).
 */
function catalogSearchCondition(query: string): SQL {
  const raw = query.toLowerCase();
  const normalized = normalizeObjectName(query).toLowerCase();
  const patterns = new Set([`%${raw}%`, `%${normalized}%`]);

  const conditions: SQL[] = [];
  for (const pattern of patterns) {
    conditions.push(
      sql`LOWER(${schema.catalogObjects.name}) LIKE ${pattern}`,
      sql`LOWER(${schema.catalogObjects.messier}) LIKE ${pattern}`,
      sql`LOWER(${schema.catalogObjects.commonNames}) LIKE ${pattern}`,
      sql`LOWER(${schema.catalogObjects.identifiers}) LIKE ${pattern}`,
    );
  }
  return sql`(${sql.join(conditions, sql` OR `)})`;
}

const NGC_CSV_URL = 'https://raw.githubusercontent.com/mattiaverga/OpenNGC/master/database_files/NGC.csv';
const ADDENDUM_CSV_URL = 'https://raw.githubusercontent.com/mattiaverga/OpenNGC/master/database_files/addendum.csv';
const GITHUB_API_URL = 'https://api.github.com/repos/mattiaverga/OpenNGC/commits?path=database_files/NGC.csv&per_page=1';

class CatalogService {
  /**
   * Download, parse, and load the OpenNGC catalog into the database.
   * Full replace strategy: delete all then insert in batches.
   */
  async loadCatalog(): Promise<{ count: number }> {
    console.log('Loading OpenNGC catalog...');

    // Fetch main NGC CSV
    const ngcResponse = await fetch(NGC_CSV_URL);
    if (!ngcResponse.ok) {
      throw new Error(`Failed to fetch NGC.csv: ${ngcResponse.status}`);
    }
    const ngcText = await ngcResponse.text();
    const ngcObjects = parseNGCCSV(ngcText);

    // Fetch addendum CSV (Messier-only objects not in NGC/IC)
    let addendumObjects: InsertCatalogObject[] = [];
    try {
      const addendumResponse = await fetch(ADDENDUM_CSV_URL);
      if (addendumResponse.ok) {
        const addendumText = await addendumResponse.text();
        addendumObjects = parseNGCCSV(addendumText);
      }
    } catch (err) {
      console.warn('Failed to fetch addendum.csv, continuing with NGC.csv only:', err);
    }

    const allObjects = [...ngcObjects, ...addendumObjects];

    // Deduplicate by name (prefer NGC entry over addendum)
    const nameMap = new Map<string, InsertCatalogObject>();
    for (const obj of allObjects) {
      if (!nameMap.has(obj.name)) {
        nameMap.set(obj.name, obj);
      }
    }
    const deduped = Array.from(nameMap.values());

    // Clear existing and bulk insert
    await db.delete(schema.catalogObjects).execute();

    const BATCH_SIZE = 500;
    for (let i = 0; i < deduped.length; i += BATCH_SIZE) {
      const batch = deduped.slice(i, i + BATCH_SIZE);
      await db.insert(schema.catalogObjects).values(batch).execute();
    }

    console.log(`Loaded ${deduped.length} catalog objects`);

    // Fetch and store the latest commit SHA
    let commitSha = '';
    try {
      const apiResponse = await fetch(GITHUB_API_URL, {
        headers: { 'User-Agent': 'Sidereal/1.0' },
      });
      if (apiResponse.ok) {
        const commits = await apiResponse.json();
        if (Array.isArray(commits) && commits.length > 0) {
          commitSha = commits[0].sha;
        }
      }
    } catch (err) {
      console.warn('Failed to fetch commit SHA:', err);
    }

    // Store catalog metadata in admin settings
    await storage.updateAdminSettings({
      catalog_lastUpdated: new Date().toISOString(),
      catalog_commitSha: commitSha,
      catalog_objectCount: deduped.length,
    });

    return { count: deduped.length };
  }

  /**
   * Check if a newer version of the catalog is available on GitHub
   */
  async checkForUpdates(): Promise<{ hasUpdate: boolean; currentSha: string; latestSha: string }> {
    const settings = await storage.getAdminSettings();
    const currentSha = (settings.catalog_commitSha as string) || '';

    const apiResponse = await fetch(GITHUB_API_URL, {
      headers: { 'User-Agent': 'Sidereal/1.0' },
    });

    if (!apiResponse.ok) {
      throw new Error(`GitHub API returned ${apiResponse.status}`);
    }

    const commits = await apiResponse.json();
    const latestSha = Array.isArray(commits) && commits.length > 0 ? commits[0].sha : '';

    return {
      hasUpdate: !!latestSha && latestSha !== currentSha,
      currentSha,
      latestSha,
    };
  }

  /**
   * Search catalog objects by name, messier designation, common names, or identifiers
   */
  async searchCatalog(query: string, limit: number = 20): Promise<CatalogObject[]> {
    return await db.select().from(schema.catalogObjects)
      .where(catalogSearchCondition(query))
      .limit(limit)
      .execute();
  }

  /**
   * Get a single catalog object by exact name match
   */
  async getCatalogObject(name: string): Promise<CatalogObject | undefined> {
    const results = await db.select().from(schema.catalogObjects)
      .where(eq(schema.catalogObjects.name, name))
      .execute();
    return results[0] || undefined;
  }

  /**
   * Given an image's tags, find matching catalog objects.
   * Returns matches sorted by priority: Messier > brightest magnitude.
   */
  async matchTargetFromTags(tags: string[]): Promise<CatalogObject[]> {
    if (!tags || tags.length === 0) return [];

    // Normalize tags for matching
    const normalizedTags = tags.map(t => normalizeObjectName(t));

    // Try exact name matches first
    const matches: CatalogObject[] = [];
    for (const tag of normalizedTags) {
      const result = await this.getCatalogObject(tag);
      if (result) {
        matches.push(result);
      }
    }

    // Also check Messier cross-references
    for (const tag of normalizedTags) {
      if (/^M\s?\d+$/i.test(tag)) {
        const pattern = tag.replace(/^M\s?/i, 'M ');
        const results = await db.select().from(schema.catalogObjects)
          .where(eq(schema.catalogObjects.messier, pattern))
          .execute();
        for (const r of results) {
          if (!matches.find(m => m.id === r.id)) {
            matches.push(r);
          }
        }
      }
    }

    return sortCatalogMatchesByPriority(matches);
  }

  /**
   * Browse catalog with pagination and filtering
   */
  async browseCatalog(options: {
    page?: number;
    limit?: number;
    q?: string;
    type?: string;
    constellation?: string;
    maxMag?: number;
    minMag?: number;
    minSize?: number;
    messierOnly?: boolean;
    sortBy?: 'name' | 'vMag' | 'majorAxis' | 'bestNow';
    sortOrder?: 'asc' | 'desc';
    latitude?: number;
    hideBelow?: boolean;
    names?: string[];
  }): Promise<{ items: CatalogObject[]; total: number; page: number; pageSize: number }> {
    const page = Math.max(1, options.page || 1);
    const limit = Math.min(100, Math.max(1, options.limit || 50));
    const offset = (page - 1) * limit;

    const conditions = [];

    if (options.q) {
      conditions.push(catalogSearchCondition(options.q));
    }

    if (options.type) {
      const types = options.type.split(',').map(t => t.trim()).filter(Boolean);
      if (types.length === 1) {
        conditions.push(eq(schema.catalogObjects.type, types[0]));
      } else if (types.length > 1) {
        conditions.push(inArray(schema.catalogObjects.type, types));
      }
    }

    if (options.constellation) {
      conditions.push(eq(schema.catalogObjects.constellation, options.constellation));
    }

    if (options.maxMag !== undefined && options.maxMag !== null) {
      conditions.push(lte(schema.catalogObjects.vMag, options.maxMag));
    }

    if (options.minMag !== undefined && options.minMag !== null) {
      conditions.push(gte(schema.catalogObjects.vMag, options.minMag));
    }

    if (options.minSize !== undefined && options.minSize !== null) {
      conditions.push(gte(schema.catalogObjects.majorAxis, options.minSize));
    }

    if (options.messierOnly) {
      conditions.push(isNotNull(schema.catalogObjects.messier));
    }

    if (options.names && options.names.length > 0) {
      conditions.push(inArray(schema.catalogObjects.name, options.names));
    }

    if (options.hideBelow && options.latitude !== undefined) {
      // Object never rises if dec is too far from observer's hemisphere
      // For northern hemisphere: dec <= -(90 - lat) means never rises
      // For southern hemisphere: dec >= (90 - |lat|) means never rises
      const poleLimit = 90 - Math.abs(options.latitude);
      if (options.latitude >= 0) {
        conditions.push(sql`${schema.catalogObjects.decDeg} > ${-poleLimit}`);
      } else {
        conditions.push(sql`${schema.catalogObjects.decDeg} < ${poleLimit}`);
      }
    }

    const whereClause = conditions.length > 0
      ? conditions.length === 1 ? conditions[0] : and(...conditions)
      : undefined;

    // Get total count
    const countResult = await db.select({ count: sql<number>`count(*)` })
      .from(schema.catalogObjects)
      .where(whereClause)
      .execute();
    const total = Number(countResult[0]?.count || 0);

    // Determine sort
    let orderClauses: SQL[];

    if (options.sortBy === 'bestNow' && options.latitude !== undefined) {
      // Best month from RA: ((round(ra_deg / 15 / 2) + 8) % 12) + 1
      const currentMonth = new Date().getMonth() + 1;
      const bestMonth = sql`((ROUND(${schema.catalogObjects.raDeg} / 15.0 / 2.0) + 8) % 12) + 1`;
      // Circular distance from current month (0 = this month is best)
      const dist = sql`MIN(ABS(${bestMonth} - ${currentMonth}), 12 - ABS(${bestMonth} - ${currentMonth}))`;
      // Never-rises last, then by month distance, then by max altitude desc
      const poleLimit = 90 - Math.abs(options.latitude);
      const neverRises = options.latitude >= 0
        ? sql`CASE WHEN ${schema.catalogObjects.decDeg} <= ${-poleLimit} THEN 1 ELSE 0 END`
        : sql`CASE WHEN ${schema.catalogObjects.decDeg} >= ${poleLimit} THEN 1 ELSE 0 END`;
      const maxAlt = sql`90.0 - ABS(${options.latitude} - ${schema.catalogObjects.decDeg})`;

      orderClauses = [
        sql`${schema.catalogObjects.raDeg} IS NULL`,
        neverRises,
        asc(dist),
        desc(maxAlt),
      ];
    } else {
      const sortCol = options.sortBy === 'vMag' ? schema.catalogObjects.vMag
        : options.sortBy === 'majorAxis' ? schema.catalogObjects.majorAxis
        : schema.catalogObjects.name;
      const sortDir = options.sortOrder === 'desc' ? desc : asc;

      orderClauses = sortCol === schema.catalogObjects.name
        ? [sortDir(sortCol)]
        : [sql`${sortCol} IS NULL`, sortDir(sortCol)];
    }

    const items = await db.select().from(schema.catalogObjects)
      .where(whereClause)
      .orderBy(...orderClauses)
      .limit(limit)
      .offset(offset)
      .execute();

    return { items, total, page, pageSize: limit };
  }

  /**
   * Get catalog status from admin settings
   */
  async getStatus(): Promise<{ count: number; lastUpdated: string | null; commitSha: string | null }> {
    const settings = await storage.getAdminSettings();
    return {
      count: (settings.catalog_objectCount as number) || 0,
      lastUpdated: (settings.catalog_lastUpdated as string) || null,
      commitSha: (settings.catalog_commitSha as string) || null,
    };
  }
}

export const catalogService = new CatalogService();
