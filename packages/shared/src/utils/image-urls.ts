export type ImageSize = 'thumbnail' | 'preview' | 'original';

export function imageUrl(id: number, size: ImageSize): string {
  return `/api/images/${id}/${size}`;
}
