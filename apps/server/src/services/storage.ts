import { db, schema, isPostgres } from '../db';
import { eq, and, inArray, lte, desc, sql } from 'drizzle-orm';
import { like, or } from 'drizzle-orm';
import { imageStorage } from './image-storage';
import type { AstroImage, InsertAstroImage, Equipment, InsertEquipment, ImageEquipment, InsertImageEquipment, PlateSolvingJob, InsertPlateSolvingJob, EquipmentGroup, InsertEquipmentGroup, EquipmentGroupMember, InsertEquipmentGroupMember, Location, InsertLocation, ImageAcquisitionRow, InsertImageAcquisitionRow, CatalogObject, InsertCatalogObject, UserTarget } from "@shared/types";
import { computeImageSummary, groupAndEnrichTargets } from '@shared/image-utils';

class DbStorage {
  // Astrophotography images
  async getAstroImages(filters?: { objectType?: string; tags?: string[]; plateSolved?: boolean; constellation?: string; equipmentId?: number }): Promise<AstroImage[]> {
    const conditions = [];

    if (filters) {
      if (filters.objectType) {
        conditions.push(eq(schema.astrophotographyImages.objectType, filters.objectType));
      }
      if (filters.plateSolved !== undefined) {
        conditions.push(eq(schema.astrophotographyImages.plateSolved, filters.plateSolved));
      }
      if (filters.constellation) {
        conditions.push(eq(schema.astrophotographyImages.constellation, filters.constellation));
      }
      if (filters.tags && filters.tags.length > 0) {
        if (isPostgres) {
          // PostgreSQL: native array overlap
          const tagArray = sql`ARRAY[${sql.join(filters.tags.map(t => sql`${t}`), sql`, `)}]::text[]`;
          conditions.push(sql`${schema.astrophotographyImages.tags} && ${tagArray}`);
        } else {
          // SQLite: tags stored as JSON array, check each tag with json_each
          const tagConditions = filters.tags.map(tag =>
            sql`EXISTS (SELECT 1 FROM json_each(${schema.astrophotographyImages.tags}) WHERE value = ${tag})`
          );
          conditions.push(or(...tagConditions)!);
        }
      }
      if (filters.equipmentId) {
        // Use a subquery instead of loading all image_equipment rows into memory
        conditions.push(
          sql`${schema.astrophotographyImages.id} IN (
            SELECT ${schema.imageEquipment.imageId} FROM ${schema.imageEquipment}
            WHERE ${schema.imageEquipment.equipmentId} = ${filters.equipmentId}
          )`
        );
      }
    }

    const whereClause = conditions.length > 0
      ? conditions.length === 1 ? conditions[0] : and(...conditions)
      : undefined;

    return await db.select().from(schema.astrophotographyImages)
      .where(whereClause)
      .execute();
  }

  async getAstroImage(id: number): Promise<AstroImage | undefined> {
    const result = await db.select().from(schema.astrophotographyImages).where(eq(schema.astrophotographyImages.id, id)).execute();
    return result[0] || undefined;
  }

  async getAstroImageByImmichId(immichId: string): Promise<AstroImage | undefined> {
    const result = await db.select().from(schema.astrophotographyImages).where(eq(schema.astrophotographyImages.immichId, immichId)).execute();
    return result[0] || undefined;
  }

  async createAstroImage(image: InsertAstroImage): Promise<AstroImage> {
    const result = await db.insert(schema.astrophotographyImages).values(image).returning().execute();
    if (!result[0]) {
      throw new Error('Failed to create astro image');
    }
    return result[0];
  }

  async updateAstroImage(id: number, updates: Partial<InsertAstroImage>): Promise<AstroImage | undefined> {
    const values = {
        ...updates,
        updatedAt: new Date(),
    };
    const result = await db.update(schema.astrophotographyImages).set(values).where(eq(schema.astrophotographyImages.id, id)).returning().execute();
    return result[0] || undefined;
  }

  async deleteAstroImage(id: number): Promise<boolean> {
    // Clean up related data (explicit for SQLite which lacks cascade, safe no-op if already cascaded in PG)
    await db.delete(schema.plateSolvingJobs).where(eq(schema.plateSolvingJobs.imageId, id)).execute();
    await db.delete(schema.imageEquipment).where(eq(schema.imageEquipment.imageId, id)).execute();
    await db.delete(schema.imageAcquisition).where(eq(schema.imageAcquisition.imageId, id)).execute();
    await db.delete(schema.astrophotographyImages).where(eq(schema.astrophotographyImages.id, id)).execute();
    // Best-effort local file cleanup — non-fatal if files don't exist
    try {
      await imageStorage.deleteImage(id);
    } catch (err: unknown) {
      console.warn(`[STORAGE] Failed to delete local files for image ${id}:`, (err as Error).message);
    }
    return true;
  }

  // Equipment
  async getEquipment(): Promise<Equipment[]> {
    const equipment = await db.select().from(schema.equipment).execute();
    return equipment;
  }

  async createEquipment(equipmentData: InsertEquipment): Promise<Equipment> {
    const values = {
        ...equipmentData,
        specifications: equipmentData.specifications,
    };
    const result = await db.insert(schema.equipment).values(values).returning().execute();
    if (!result[0]) {
      throw new Error('Failed to create equipment');
    }
    return result[0];
  }

  async updateEquipment(id: number, updates: Partial<InsertEquipment>): Promise<Equipment | undefined> {
    const values = {
        ...updates,
        specifications: updates.specifications,
        updatedAt: new Date(),
    };
    const result = await db.update(schema.equipment).set(values).where(eq((schema.equipment).id, id)).returning().execute();
    return result[0] ? result[0] : undefined;
  }

  async deleteEquipment(id: number): Promise<boolean> {
    await db.delete(schema.equipment).where(eq((schema.equipment).id, id)).execute();
    return true;
  }

  // Image Equipment relationships
  async getImageEquipment(imageId: number): Promise<ImageEquipment[]> {
    const imageEquipment = await db.select().from(schema.imageEquipment).where(eq((schema.imageEquipment).imageId, imageId)).execute();
    return imageEquipment;
  }

  async getEquipmentForImage(imageId: number): Promise<Equipment[]> {
    const imageEquipment = await this.getImageEquipment(imageId);
    const equipmentIds = imageEquipment.map(ie => ie.equipmentId);
    if (equipmentIds.length === 0) {
        return [];
    }
    const equipment = await db.select().from(schema.equipment).where(inArray((schema.equipment).id, equipmentIds)).execute();
    return equipment;
  }

  async addEquipmentToImage(imageId: number, equipmentId: number, settings?: Record<string, unknown>, notes?: string): Promise<ImageEquipment> {
    const values = {
        imageId,
        equipmentId,
        settings: settings,
        notes,
    };
    const result = await db.insert(schema.imageEquipment).values(values).returning().execute();
    if (!result[0]) {
      throw new Error('Failed to add equipment to image');
    }
    await this.recomputeImageSummary(imageId);
    return result[0];
  }

  async removeEquipmentFromImage(imageId: number, equipmentId: number): Promise<boolean> {
    await db.delete(schema.imageEquipment).where(and(eq((schema.imageEquipment).imageId, imageId), eq((schema.imageEquipment).equipmentId, equipmentId))).execute();
    await this.recomputeImageSummary(imageId);
    return true;
  }

  async updateImageEquipment(imageId: number, equipmentId: number, updates: Partial<InsertImageEquipment>): Promise<ImageEquipment | undefined> {
    const values = {
        ...updates,
        settings: updates.settings,
    };
    const result = await db.update(schema.imageEquipment).set(values).where(and(eq((schema.imageEquipment).imageId, imageId), eq((schema.imageEquipment).equipmentId, equipmentId))).returning().execute();
    return result[0] ? result[0] : undefined;
  }

  // Equipment Groups
  async getEquipmentGroups(): Promise<(EquipmentGroup & { members: Equipment[] })[]> {
    const groups: EquipmentGroup[] = await db.select().from(schema.equipmentGroups).execute();
    if (groups.length === 0) return [];

    const groupIds = groups.map((g: EquipmentGroup) => g.id);

    // Fetch all memberships and equipment in 2 queries instead of 2N
    const allMemberRows = await db.select().from(schema.equipmentGroupMembers)
      .where(inArray(schema.equipmentGroupMembers.groupId, groupIds)).execute();

    const allEquipmentIds = [...new Set(allMemberRows.map((m: EquipmentGroupMember) => m.equipmentId))];
    let allEquipment: Equipment[] = [];
    if (allEquipmentIds.length > 0) {
      allEquipment = await db.select().from(schema.equipment)
        .where(inArray(schema.equipment.id, allEquipmentIds)).execute();
    }

    const equipmentById = new Map(allEquipment.map(e => [e.id, e]));
    const membersByGroupId = new Map<number, Equipment[]>();
    for (const m of allMemberRows) {
      const eq = equipmentById.get(m.equipmentId);
      if (eq) {
        const list = membersByGroupId.get(m.groupId) || [];
        list.push(eq);
        membersByGroupId.set(m.groupId, list);
      }
    }

    return groups.map((group: EquipmentGroup) => ({
      ...group,
      members: membersByGroupId.get(group.id) || [],
    }));
  }

  async getEquipmentGroup(id: number): Promise<(EquipmentGroup & { members: Equipment[] }) | undefined> {
    const groups = await db.select().from(schema.equipmentGroups)
      .where(eq(schema.equipmentGroups.id, id)).execute();
    if (!groups[0]) return undefined;

    const memberRows = await db.select().from(schema.equipmentGroupMembers)
      .where(eq(schema.equipmentGroupMembers.groupId, id)).execute();
    const equipmentIds = memberRows.map((m: EquipmentGroupMember) => m.equipmentId);
    let members: Equipment[] = [];
    if (equipmentIds.length > 0) {
      members = await db.select().from(schema.equipment)
        .where(inArray(schema.equipment.id, equipmentIds)).execute();
    }
    return { ...groups[0], members };
  }

  async createEquipmentGroup(data: { name: string; description?: string; memberIds?: number[] }): Promise<EquipmentGroup> {
    const result = await db.insert(schema.equipmentGroups).values({
      name: data.name,
      description: data.description || null,
    }).returning().execute();
    if (!result[0]) throw new Error('Failed to create equipment group');
    if (data.memberIds && data.memberIds.length > 0) {
      const memberValues = data.memberIds.map(equipmentId => ({
        groupId: result[0].id,
        equipmentId,
      }));
      await db.insert(schema.equipmentGroupMembers).values(memberValues).execute();
    }
    return result[0];
  }

  async updateEquipmentGroup(id: number, updates: { name?: string; description?: string }): Promise<EquipmentGroup | undefined> {
    const values = { ...updates, updatedAt: new Date() };
    const result = await db.update(schema.equipmentGroups).set(values)
      .where(eq(schema.equipmentGroups.id, id)).returning().execute();
    return result[0] || undefined;
  }

  async deleteEquipmentGroup(id: number): Promise<boolean> {
    await db.delete(schema.equipmentGroupMembers).where(eq(schema.equipmentGroupMembers.groupId, id)).execute();
    await db.delete(schema.equipmentGroups).where(eq(schema.equipmentGroups.id, id)).execute();
    return true;
  }

  async setEquipmentGroupMembers(groupId: number, memberIds: number[]): Promise<void> {
    await db.delete(schema.equipmentGroupMembers).where(eq(schema.equipmentGroupMembers.groupId, groupId)).execute();
    if (memberIds.length > 0) {
      const values = memberIds.map(equipmentId => ({ groupId, equipmentId }));
      await db.insert(schema.equipmentGroupMembers).values(values).execute();
    }
  }

  async applyEquipmentGroupToImage(groupId: number, imageId: number): Promise<Equipment[]> {
    const memberRows = await db.select().from(schema.equipmentGroupMembers)
      .where(eq(schema.equipmentGroupMembers.groupId, groupId)).execute();
    const existingRows = await db.select().from(schema.imageEquipment)
      .where(eq(schema.imageEquipment.imageId, imageId)).execute();
    const existingIds = new Set(existingRows.map((r: ImageEquipment) => r.equipmentId));
    const newIds = memberRows
      .map((m: EquipmentGroupMember) => m.equipmentId)
      .filter((id: number) => !existingIds.has(id));
    if (newIds.length > 0) {
      const values = newIds.map((equipmentId: number) => ({ imageId, equipmentId }));
      await db.insert(schema.imageEquipment).values(values).execute();
      await this.recomputeImageSummary(imageId);
    }
    if (newIds.length === 0) return [];
    return await db.select().from(schema.equipment).where(inArray(schema.equipment.id, newIds)).execute();
  }

  // Plate solving
  async getPlateSolvingJobs(): Promise<PlateSolvingJob[]> {
    const jobs = await db.select().from(schema.plateSolvingJobs).execute();
    return jobs;
  }

  async getPlateSolvingJob(id: number): Promise<PlateSolvingJob | undefined> {
    const result = await db.select().from(schema.plateSolvingJobs).where(eq((schema.plateSolvingJobs).id, id)).execute();
    return result[0] ? result[0] : undefined;
  }

  async getPlateSolvingJobByAstrometryId(astrometryJobId: string): Promise<PlateSolvingJob | undefined> {
    const result = await db.select().from(schema.plateSolvingJobs).where(eq((schema.plateSolvingJobs).astrometryJobId, astrometryJobId)).execute();
    return result[0] ? result[0] : undefined;
  }

  async getLatestPlateSolvingJob(imageId: number, status?: string): Promise<PlateSolvingJob | undefined> {
    const conditions = [eq(schema.plateSolvingJobs.imageId, imageId)];
    if (status) {
      conditions.push(eq(schema.plateSolvingJobs.status, status));
    }
    const result = await db.select().from(schema.plateSolvingJobs)
      .where(and(...conditions))
      .orderBy(desc(schema.plateSolvingJobs.submittedAt))
      .limit(1)
      .execute();
    return result[0] || undefined;
  }

  async createPlateSolvingJob(job: InsertPlateSolvingJob): Promise<PlateSolvingJob> {
    const values = {
        ...job,
        result: job.result,
    };
    const result = await db.insert(schema.plateSolvingJobs).values(values).returning().execute();
    if (!result[0]) {
      throw new Error('Failed to create plate solving job');
    }
    return result[0];
  }

  async updatePlateSolvingJob(id: number, updates: Partial<InsertPlateSolvingJob>): Promise<PlateSolvingJob | undefined> {
    const values = {
        ...updates,
        result: updates.result,
        completedAt: new Date(),
    };
    const result = await db.update(schema.plateSolvingJobs).set(values).where(eq((schema.plateSolvingJobs).id, id)).returning().execute();
    return result[0] ? result[0] : undefined;
  }

  // Admin settings
  async getAdminSettings(): Promise<Record<string, unknown>> {
    const settings = await db.select().from(schema.adminSettings).execute();
    return settings.reduce((acc: Record<string, unknown>, setting: { key: string; value: unknown }) => {
      acc[setting.key] = setting.value;
      return acc;
    }, {} as Record<string, unknown>);
  }

  async updateAdminSettings(settings: Record<string, unknown>): Promise<void> {
    for (const key in settings) {
      const value = settings[key];
      await db.insert(schema.adminSettings)
        .values({ key, value })
        .onConflictDoUpdate({ target: (schema.adminSettings).key, set: { value } })
        .execute();
    }
  }

  // Notifications
  async getNotifications() {
    const notifications = await db.select().from(schema.notifications).where(eq((schema.notifications).acknowledged, false)).execute();
    return notifications;
  }

  async createNotification(notification: { type: string; title: string; message: string; details?: unknown }) {
    const values = {
        ...notification,
        details: notification.details,
    };
    const result = await db.insert(schema.notifications).values(values).returning().execute();
    return result[0] || undefined;
  }

  async acknowledgeNotification(id: number): Promise<void> {
    await db.update(schema.notifications).set({ acknowledged: true }).where(eq((schema.notifications).id, id)).execute();
  }

  async acknowledgeAllNotifications(): Promise<void> {
    await db.update(schema.notifications).set({ acknowledged: true }).where(eq((schema.notifications).acknowledged, false)).execute();
  }

  async clearOldNotifications(daysOld: number = 30): Promise<void> {
    const cutoffDate = new Date();
    cutoffDate.setDate(cutoffDate.getDate() - daysOld);
    await db.delete(schema.notifications).where(and(eq(schema.notifications.acknowledged, true), lte(schema.notifications.createdAt, cutoffDate))).execute();
  }

  // Locations
  async getLocations(): Promise<Location[]> {
    return await db.select().from(schema.locations).execute();
  }

  async getLocation(id: number): Promise<Location | undefined> {
    const result = await db.select().from(schema.locations).where(eq(schema.locations.id, id)).execute();
    return result[0] || undefined;
  }

  async createLocation(location: InsertLocation): Promise<Location> {
    const result = await db.insert(schema.locations).values(location).returning().execute();
    if (!result[0]) {
      throw new Error('Failed to create location');
    }
    return result[0];
  }

  async updateLocation(id: number, updates: Partial<InsertLocation>): Promise<Location | undefined> {
    const values = {
      ...updates,
      updatedAt: new Date(),
    };
    const result = await db.update(schema.locations).set(values).where(eq(schema.locations.id, id)).returning().execute();
    return result[0] || undefined;
  }

  async deleteLocation(id: number): Promise<boolean> {
    await db.delete(schema.locations).where(eq(schema.locations.id, id)).execute();
    return true;
  }

  // Image Acquisition entries
  async getImageAcquisitions(imageId: number): Promise<ImageAcquisitionRow[]> {
    return await db.select().from(schema.imageAcquisition).where(eq(schema.imageAcquisition.imageId, imageId)).execute();
  }

  async createImageAcquisition(data: InsertImageAcquisitionRow): Promise<ImageAcquisitionRow> {
    const result = await db.insert(schema.imageAcquisition).values(data).returning().execute();
    if (!result[0]) {
      throw new Error('Failed to create acquisition entry');
    }
    await this.recomputeImageSummary(data.imageId);
    return result[0];
  }

  async updateImageAcquisition(id: number, updates: Partial<InsertImageAcquisitionRow>): Promise<ImageAcquisitionRow | undefined> {
    const result = await db.update(schema.imageAcquisition).set(updates).where(eq(schema.imageAcquisition.id, id)).returning().execute();
    if (result[0]) {
      await this.recomputeImageSummary(result[0].imageId);
    }
    return result[0] || undefined;
  }

  async deleteImageAcquisition(id: number): Promise<boolean> {
    const existing = await db.select().from(schema.imageAcquisition).where(eq(schema.imageAcquisition.id, id)).execute();
    if (!existing[0]) return false;

    await db.delete(schema.imageAcquisition).where(eq(schema.imageAcquisition.id, id)).execute();
    await this.recomputeImageSummary(existing[0].imageId);
    return true;
  }

  // Recompute flat summary fields from acquisition data + linked equipment
  async recomputeImageSummary(imageId: number): Promise<void> {
    const acquisitions = await this.getImageAcquisitions(imageId);
    const linkedEquipment = await this.getEquipmentForImage(imageId);

    const updates = computeImageSummary(
      acquisitions.map(a => ({ frameCount: a.frameCount, exposureTime: a.exposureTime, filterName: a.filterName })),
      linkedEquipment.map(e => ({ type: e.type, specifications: e.specifications as Record<string, unknown> | null })),
    );

    if (Object.keys(updates).length > 0) {
      await this.updateAstroImage(imageId, updates);
    }
  }

  // Catalog objects
  async getCatalogObjects(): Promise<CatalogObject[]> {
    return await db.select().from(schema.catalogObjects).execute();
  }

  async getCatalogObject(name: string): Promise<CatalogObject | undefined> {
    const result = await db.select().from(schema.catalogObjects).where(eq(schema.catalogObjects.name, name)).execute();
    return result[0] || undefined;
  }

  async searchCatalogObjects(query: string, limit: number = 20): Promise<CatalogObject[]> {
    const pattern = `%${query}%`;
    return await db.select().from(schema.catalogObjects)
      .where(
        or(
          like(schema.catalogObjects.name, pattern),
          like(schema.catalogObjects.messier, pattern),
          like(schema.catalogObjects.commonNames, pattern),
          like(schema.catalogObjects.identifiers, pattern),
        )
      )
      .limit(limit)
      .execute();
  }

  async bulkInsertCatalogObjects(objects: InsertCatalogObject[]): Promise<void> {
    const BATCH_SIZE = 500;
    for (let i = 0; i < objects.length; i += BATCH_SIZE) {
      const batch = objects.slice(i, i + BATCH_SIZE);
      await db.insert(schema.catalogObjects).values(batch).execute();
    }
  }

  async clearCatalogObjects(): Promise<void> {
    await db.delete(schema.catalogObjects).execute();
  }

  // User Targets (annotations on catalog objects)
  async getUserTargets(): Promise<UserTarget[]> {
    return await db.select().from(schema.userTargets).execute();
  }

  async getUserTarget(catalogName: string): Promise<UserTarget | undefined> {
    const result = await db.select().from(schema.userTargets).where(eq(schema.userTargets.catalogName, catalogName)).execute();
    return result[0] || undefined;
  }

  async upsertUserTarget(catalogName: string, data: { notes?: string | null; tags?: string[] | null }): Promise<UserTarget> {
    const existing = await this.getUserTarget(catalogName);
    if (existing) {
      const result = await db.update(schema.userTargets).set({
        notes: data.notes !== undefined ? data.notes : existing.notes,
        tags: data.tags !== undefined ? data.tags : existing.tags,
        updatedAt: new Date(),
      }).where(eq(schema.userTargets.catalogName, catalogName)).returning().execute();
      return result[0];
    }
    const result = await db.insert(schema.userTargets).values({
      catalogName,
      notes: data.notes || null,
      tags: data.tags || null,
    }).returning().execute();
    if (!result[0]) throw new Error('Failed to create user target');
    return result[0];
  }

  async deleteUserTarget(catalogName: string): Promise<boolean> {
    await db.delete(schema.userTargets).where(eq(schema.userTargets.catalogName, catalogName)).execute();
    return true;
  }

  // Targets (images grouped by target name)
  async getTargets() {
    const images = await this.getAstroImages();
    const catalogObjects = await this.getCatalogObjects();

    return groupAndEnrichTargets(images, catalogObjects);
  }

  // Stats
  async getStats() {
    try {
      // Use SQL aggregation instead of loading all rows into memory
      const img = schema.astrophotographyImages;
      const jobs = schema.plateSolvingJobs;

      const [imageStats] = await db.select({
        totalImages: sql<number>`count(*)`,
        plateSolved: sql<number>`sum(case when ${img.plateSolved} = true then 1 else 0 end)`,
        totalHours: sql<number>`coalesce(sum(${img.totalIntegration}), 0)`,
        uniqueTargets: sql<number>`count(distinct case when ${img.title} is not null and ${img.title} != '' then ${img.title} end)`,
      }).from(img).execute();

      const [equipmentCount] = await db.select({
        total: sql<number>`count(*)`,
      }).from(schema.equipment).execute();

      const [jobStats] = await db.select({
        total: sql<number>`count(*)`,
        pending: sql<number>`sum(case when ${jobs.status} = 'pending' then 1 else 0 end)`,
        successful: sql<number>`sum(case when ${jobs.status} = 'success' then 1 else 0 end)`,
        failed: sql<number>`sum(case when ${jobs.status} = 'failed' then 1 else 0 end)`,
      }).from(jobs).execute();

      // Object type distribution still needs grouping
      const typeRows = await db.select({
        objectType: sql<string>`coalesce(${img.objectType}, 'Unknown')`,
        count: sql<number>`count(*)`,
      }).from(img).groupBy(sql`coalesce(${img.objectType}, 'Unknown')`).execute();

      const objectTypeCounts: Record<string, number> = {};
      for (const row of typeRows) {
        objectTypeCounts[row.objectType] = Number(row.count);
      }

      return {
        totalImages: Number(imageStats.totalImages),
        plateSolved: Number(imageStats.plateSolved),
        totalHours: Math.round(Number(imageStats.totalHours) * 100) / 100,
        uniqueTargets: Number(imageStats.uniqueTargets),
        totalEquipment: Number(equipmentCount.total),
        objectTypeCounts,
        plateSolvingStats: {
          total: Number(jobStats.total),
          pending: Number(jobStats.pending),
          successful: Number(jobStats.successful),
          failed: Number(jobStats.failed),
        }
      };
    } catch (error) {
      console.error("Failed to get stats:", error);
      throw error;
    }
  }
}

export const storage = new DbStorage();
