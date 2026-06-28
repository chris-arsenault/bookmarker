import { useEffect, type Dispatch, type SetStateAction } from "react";
import type { ApiClient } from "./api";
import type { AuthState } from "./auth";
import { mergePreviewUrls, previewUrlsForItems, type PreviewUrls } from "./itemPreviewUrls";
import { libraryFiltersToApiFilters, type LibraryFilters } from "./libraryFilters";
import type { LibraryState } from "./libraryState";
import { applyLibraryUpdates, updateCursorString, updatePollInterval } from "./libraryUpdates";

export function useLibraryUpdatePoller({
  apiClient,
  authStatus,
  filters,
  libraryState,
  updatesCursor,
  setUpdatesCursor,
  setLibraryState,
  setThumbnailUrls,
}: {
  apiClient: ApiClient;
  authStatus: AuthState["status"];
  filters: LibraryFilters;
  libraryState: LibraryState;
  updatesCursor: string | null;
  setUpdatesCursor: (cursor: string) => void;
  setLibraryState: Dispatch<SetStateAction<LibraryState>>;
  setThumbnailUrls: Dispatch<SetStateAction<PreviewUrls>>;
}) {
  const readyItems = libraryState.status === "ready" ? libraryState.items : null;
  const pollMs = readyItems ? updatePollInterval(readyItems) : null;

  useEffect(() => {
    if (authStatus !== "signed-in" || !updatesCursor || !pollMs) {
      return;
    }
    let stopped = false;
    let inFlight = false;
    const tick = async () => {
      if (inFlight || documentHidden()) {
        return;
      }
      inFlight = true;
      try {
        await pollLibraryUpdates({
          apiClient,
          filters,
          updatesCursor,
          setUpdatesCursor,
          setLibraryState,
          setThumbnailUrls,
        });
      } finally {
        inFlight = false;
      }
    };
    const interval = window.setInterval(() => {
      if (!stopped) {
        tick().catch(() => {});
      }
    }, pollMs);
    return () => {
      stopped = true;
      window.clearInterval(interval);
    };
  }, [
    apiClient,
    authStatus,
    filters,
    pollMs,
    setLibraryState,
    setThumbnailUrls,
    setUpdatesCursor,
    updatesCursor,
  ]);
}

async function pollLibraryUpdates({
  apiClient,
  filters,
  updatesCursor,
  setUpdatesCursor,
  setLibraryState,
  setThumbnailUrls,
}: {
  apiClient: ApiClient;
  filters: LibraryFilters;
  updatesCursor: string;
  setUpdatesCursor: (cursor: string) => void;
  setLibraryState: Dispatch<SetStateAction<LibraryState>>;
  setThumbnailUrls: Dispatch<SetStateAction<PreviewUrls>>;
}) {
  const updates = await apiClient.listItemUpdates({
    ...libraryFiltersToApiFilters(filters),
    since: updatesCursor,
    limit: 100,
  });
  setUpdatesCursor(updateCursorString(updates.cursor));
  setLibraryState((state) => applyLibraryUpdates(state, updates));
  if (updates.items.length === 0) {
    return;
  }
  const previewUrls = await previewUrlsForItems(apiClient, updates.items);
  setThumbnailUrls((current) => mergePreviewUrls(current, updates.items, previewUrls));
}

function documentHidden() {
  return typeof document !== "undefined" && document.visibilityState === "hidden";
}
