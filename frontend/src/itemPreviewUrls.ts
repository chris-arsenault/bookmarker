import type { ApiClient } from "./api";
import type { LibraryItemSummary } from "./types";

export type PreviewUrls = Record<string, string>;

export async function previewUrlsForItems(
  apiClient: ApiClient,
  items: LibraryItemSummary[]
): Promise<PreviewUrls> {
  const entries = await Promise.all(
    items.filter(hasArchivedThumbnail).map((item) => previewUrlEntry(apiClient, item))
  );
  return Object.fromEntries(entries.filter(isPreviewUrlEntry));
}

export function mergePreviewUrls(
  current: PreviewUrls,
  items: LibraryItemSummary[],
  previewUrls: PreviewUrls
) {
  const next = { ...current, ...previewUrls };
  for (const item of items) {
    if (!hasArchivedThumbnail(item)) {
      delete next[item.id];
    }
  }
  return next;
}

async function previewUrlEntry(
  apiClient: ApiClient,
  item: LibraryItemSummary
): Promise<[string, string] | null> {
  const blob = await apiClient.fetchThumbnail(item.id).catch(() => null);
  return blob ? [item.id, URL.createObjectURL(blob)] : null;
}

function hasArchivedThumbnail(item: LibraryItemSummary) {
  return Boolean(item.thumbnail_s3_key);
}

function isPreviewUrlEntry(entry: [string, string] | null): entry is [string, string] {
  return entry !== null;
}
