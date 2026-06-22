import type { Dispatch, SetStateAction } from "react";
import type { ApiClient } from "./api";
import {
  captureLinkItem,
  captureTextItem,
  copyLink,
  deleteLibraryItem,
  mergeLibraryTags,
  openSource,
  renameLibraryTag,
  selectItem,
  updateItemOrganization,
} from "./libraryActions";
import { LibraryView } from "./LibraryView";
import type { LibraryFilters } from "./libraryFilters";
import type { LibraryState } from "./libraryState";

type LibraryViewShellProps = {
  apiClient: ApiClient;
  filters: LibraryFilters;
  libraryState: LibraryState;
  thumbnailUrls: Record<string, string>;
  setFilters: Dispatch<SetStateAction<LibraryFilters>>;
  setLibraryState: Dispatch<SetStateAction<LibraryState>>;
  setThumbnailUrls: Dispatch<SetStateAction<Record<string, string>>>;
  setUpdatesCursor: (cursor: string) => void;
};

export function LibraryViewShell({
  apiClient,
  filters,
  libraryState,
  thumbnailUrls,
  setFilters,
  setLibraryState,
  setThumbnailUrls,
  setUpdatesCursor,
}: LibraryViewShellProps) {
  return (
    <LibraryView
      filters={filters}
      state={libraryState}
      thumbnailUrls={thumbnailUrls}
      onCreateLink={(request) =>
        captureLinkItem(
          apiClient,
          request,
          setFilters,
          setLibraryState,
          setThumbnailUrls,
          setUpdatesCursor
        )
      }
      onCreateText={(request) =>
        captureTextItem(
          apiClient,
          request,
          setFilters,
          setLibraryState,
          setThumbnailUrls,
          setUpdatesCursor
        )
      }
      onDeleteItem={(itemId) =>
        deleteLibraryItem(
          apiClient,
          filters,
          itemId,
          setLibraryState,
          setThumbnailUrls,
          setUpdatesCursor
        )
      }
      onFiltersChange={setFilters}
      onSelectItem={(itemId) => {
        selectItem(apiClient, itemId, setLibraryState).catch(() => {});
      }}
      onCopyLink={copyLink}
      onLoadImage={(itemId) => apiClient.fetchImage(itemId)}
      onOpenSource={openSource}
      onUpdateItem={(itemId, request) =>
        updateItemOrganization(apiClient, itemId, request, setLibraryState)
      }
      onRenameTag={(tagId, request) =>
        renameLibraryTag(apiClient, filters, libraryState, tagId, request, setLibraryState)
      }
      onMergeTags={(tagId, request) =>
        mergeLibraryTags(apiClient, filters, libraryState, tagId, request, setLibraryState)
      }
    />
  );
}
