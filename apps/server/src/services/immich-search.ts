type SearchResponse = { json(): Promise<unknown> };
type SearchFetch = (url: string, init: { method: string; headers: Record<string, string>; body: string }) => Promise<SearchResponse>;

/**
 * Page through Immich's POST /api/search/metadata and return every matching image asset.
 * `filter` is merged into the request body (e.g. `{ albumIds: [id] }`).
 */
export async function searchImmichAssets(
  config: { host: string; apiKey: string },
  filter: Record<string, unknown> = {},
  fetchFn: SearchFetch = fetch,
): Promise<Record<string, unknown>[]> {
  const results: Record<string, unknown>[] = [];
  let page = 1;
  const pageSize = 1000;
  let hasMore = true;

  while (hasMore) {
    const response = await fetchFn(`${config.host}/api/search/metadata`, {
      method: 'POST',
      headers: { 'X-API-Key': config.apiKey, 'Content-Type': 'application/json' },
      body: JSON.stringify({ ...filter, size: pageSize, page, type: 'IMAGE' }),
    });
    const data = await response.json() as Record<string, unknown>;

    const assets = data?.assets as Record<string, unknown> | undefined;
    const items = (assets?.items || []) as Record<string, unknown>[];
    results.push(...items);

    // Immich returns nextPage as a string (e.g. "2"), or null on the last page.
    const nextPage = assets?.nextPage;
    if (nextPage != null && items.length > 0) {
      page = Number(nextPage);
    } else {
      hasMore = false;
    }
  }

  return results;
}
