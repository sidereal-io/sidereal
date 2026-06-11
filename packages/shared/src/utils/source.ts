type SourceFields = {
  immichId?: string | null;
  sourceType?: string | null;
  sourceId?: string | null;
};

/**
 * Fills source_type/source_id from immich_id for backwards-compatible dual-write.
 * Explicitly-provided source fields are never overridden.
 */
export function withSourceDefaults<T extends SourceFields>(image: T): T {
  const sourceType = image.sourceType ?? 'immich';
  const sourceId = image.sourceId ?? image.immichId ?? undefined;
  return { ...image, sourceType, sourceId };
}
