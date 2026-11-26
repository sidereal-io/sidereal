// One Asset as the Library shows it.
export interface Entry {
  id: string;
  name: string;
  originalUrl: string;
  // The address the image shows from: its preview, or else its original.
  imageUrl: string;
}

// An original is always fetched from this server's API, never from an
// address the list names elsewhere, and only from its own Asset's route.
const ORIGINAL_URL = /^\/api\/assets\/[\w-]+\/original$/;

export function isObject(body: unknown): body is Record<string, unknown> {
  return typeof body === "object" && body !== null;
}

// The address an Asset's image shows from, given its id and its
// `preview_url`: the preview, when it has one, or else the original. Null
// when `preview_url` is neither null nor this Asset's own preview route.
export function imageUrlOf(id: string, previewUrl: unknown): string | null {
  if (previewUrl === null) return `/api/assets/${id}/original`;
  if (previewUrl === `/api/assets/${id}/preview`) return previewUrl;
  return null;
}

// One Asset entry, as the list and a repeat upload's answer hold it, or
// null when it is not a valid entry. Nothing else from it is kept, and the
// name is only ever shown as text.
export function toEntry(item: unknown): Entry | null {
  if (
    !isObject(item) ||
    typeof item.id !== "string" ||
    typeof item.name !== "string" ||
    typeof item.current_version_id !== "string" ||
    typeof item.original_url !== "string" ||
    !ORIGINAL_URL.test(item.original_url) ||
    item.original_url !== `/api/assets/${item.id}/original`
  ) {
    return null;
  }
  const imageUrl = imageUrlOf(item.id, item.preview_url);
  if (imageUrl === null) return null;
  return {
    id: item.id,
    name: item.name,
    originalUrl: item.original_url,
    imageUrl,
  };
}
