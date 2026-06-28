import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import type { ApiClient } from "./api";
import type { LibraryActions } from "./LibraryActionsContext";
import {
  captureLinkItem,
  captureTextItem,
  copyLink,
  deleteLibraryItem,
  loadLibrary,
  mergeLibraryTags,
  openSource,
  renameLibraryTag,
  selectItem,
  updateItemOrganization,
} from "./libraryActions";
import type { LibraryFilters } from "./libraryFilters";
import { clearSelectedItem, type LibraryState } from "./libraryState";
import type { PreviewUrls } from "./itemPreviewUrls";
import type { ImageAccessLoader } from "./ImageAccessContext";
import { useLibraryUpdatePoller } from "./useLibraryUpdatePoller";

export type LibrarySnapshot = {
  filters: LibraryFilters;
  state: LibraryState;
  thumbnailUrls: PreviewUrls;
};

type LatestRef<T> = {
  current: T;
};

type ControllerActionState = {
  apiClient: ApiClient;
  filtersRef: LatestRef<LibraryFilters>;
  setFilters: Dispatch<SetStateAction<LibraryFilters>>;
  setState: Dispatch<SetStateAction<LibraryState>>;
  setThumbnailUrls: Dispatch<SetStateAction<PreviewUrls>>;
  setUpdatesCursor: Dispatch<SetStateAction<string | null>>;
  stateRef: LatestRef<LibraryState>;
};

export function useLibraryController(apiClient: ApiClient) {
  const [state, setState] = useState<LibraryState>({ status: "loading" });
  const [filters, setFilters] = useState<LibraryFilters>({});
  const [thumbnailUrls, setThumbnailUrls] = useState<PreviewUrls>({});
  const [updatesCursor, setUpdatesCursor] = useState<string | null>(null);
  const stateRef = useLatestRef(state);
  const filtersRef = useLatestRef(filters);

  const refreshLibrary = useCallback(
    () => loadLibrary(apiClient, filters, setState, setThumbnailUrls, setUpdatesCursor),
    [apiClient, filters]
  );

  useEffect(() => {
    refreshLibrary().catch(() => {});
  }, [refreshLibrary]);

  useLibraryUpdatePoller({
    apiClient,
    filters,
    libraryState: state,
    updatesCursor,
    setUpdatesCursor,
    setLibraryState: setState,
    setThumbnailUrls,
  });

  const loadImageAccess = useCallback<ImageAccessLoader>(
    (itemId) => apiClient.getImageAccess(itemId),
    [apiClient]
  );

  return {
    actions: useControllerActions({
      apiClient,
      filtersRef,
      setFilters,
      setState,
      setThumbnailUrls,
      setUpdatesCursor,
      stateRef,
    }),
    loadImageAccess,
    snapshot: useMemo(() => ({ filters, state, thumbnailUrls }), [filters, state, thumbnailUrls]),
  };
}

function useControllerActions({
  apiClient,
  filtersRef,
  setFilters,
  setState,
  setThumbnailUrls,
  setUpdatesCursor,
  stateRef,
}: ControllerActionState) {
  return useMemo<LibraryActions>(
    () => ({
      changeFilters: setFilters,
      closeDetail: () => setState(clearSelectedItem),
      copyItem: copyLink,
      createLink: (request) =>
        captureLinkItem(
          apiClient,
          request,
          setFilters,
          setState,
          setThumbnailUrls,
          setUpdatesCursor
        ),
      createText: (request) =>
        captureTextItem(
          apiClient,
          request,
          setFilters,
          setState,
          setThumbnailUrls,
          setUpdatesCursor
        ),
      deleteItem: (itemId) =>
        deleteLibraryItem(
          apiClient,
          filtersRef.current,
          itemId,
          setState,
          setThumbnailUrls,
          setUpdatesCursor
        ),
      mergeTags: (sourceTagId, request) =>
        mergeLibraryTags(
          apiClient,
          filtersRef.current,
          stateRef.current,
          sourceTagId,
          request,
          setState
        ),
      openSource,
      renameTag: (tagId, request) =>
        renameLibraryTag(apiClient, filtersRef.current, stateRef.current, tagId, request, setState),
      selectItem: (itemId) => {
        selectItem(apiClient, itemId, setState).catch(() => {});
      },
      updateItem: (itemId, request) => updateItemOrganization(apiClient, itemId, request, setState),
    }),
    [apiClient, filtersRef, setFilters, setState, setThumbnailUrls, setUpdatesCursor, stateRef]
  );
}

function useLatestRef<T>(value: T) {
  const ref = useRef(value);
  useEffect(() => {
    ref.current = value;
  }, [value]);
  return ref;
}
