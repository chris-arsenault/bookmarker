import type { ApiClient } from "./api";
import { previewUrlsForItems } from "./itemPreviewUrls";
import { libraryFiltersToApiFilters, type LibraryFilters } from "./libraryFilters";
import { updateCursorString } from "./libraryUpdates";

export async function loadLibraryData(apiClient: ApiClient, filters: LibraryFilters) {
  const [items, tags, updates] = await Promise.all([
    apiClient.listItems(libraryFiltersToApiFilters(filters)),
    apiClient.listTags(),
    apiClient.listItemUpdates(),
  ]);
  return {
    items,
    tags,
    thumbnailUrls: await previewUrlsForItems(apiClient, items),
    updatesCursor: updateCursorString(updates.cursor),
  };
}
