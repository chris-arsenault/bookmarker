import type { ApiClient } from "./api";
import { libraryFiltersToApiFilters, type LibraryFilters } from "./libraryFilters";
import { updateCursorString } from "./libraryUpdates";
import type { LibraryItemSummary } from "./types";

export async function loadLibraryData(apiClient: ApiClient, filters: LibraryFilters) {
  const [items, tags, updates] = await Promise.all([
    apiClient.listItems(libraryFiltersToApiFilters(filters)),
    apiClient.listTags(),
    apiClient.listItemUpdates(),
  ]);
  return {
    items,
    tags,
    thumbnailUrls: await thumbnailUrlsForItems(apiClient, items),
    updatesCursor: updateCursorString(updates.cursor),
  };
}

async function thumbnailUrlsForItems(apiClient: ApiClient, items: LibraryItemSummary[]) {
  const entries = await Promise.all(
    items
      .filter((item) => item.thumbnail_s3_key)
      .map(async (item) => [item.id, URL.createObjectURL(await apiClient.fetchThumbnail(item.id))])
  );
  return Object.fromEntries(entries);
}
