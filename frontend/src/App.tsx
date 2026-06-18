import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { ApiClient } from "./api";
import { createAuthClient, type AuthClient, type AuthState } from "./auth";
import { copyCanonicalLink } from "./itemActions";
import { LibraryView } from "./LibraryView";
import { libraryFiltersToApiFilters, type LibraryFilters } from "./libraryFilters";
import {
  applyUpdatedItemSummary,
  readyLibraryState,
  replaceSelectedDetail,
  replaceTagCorpus,
  selectLibraryItem,
  type LibraryState,
} from "./libraryState";
import type {
  CaptureLinkRequest,
  CaptureTextRequest,
  LibraryItemDetail,
  LibraryItemSummary,
  MergeTagsRequest,
  RenameTagRequest,
  TagCorpusEntry,
  UpdateItemRequest,
} from "./types";
import { SignInPanel } from "./SignInPanel";

const authClient = createAuthClient();

export function App() {
  const apiClient = useMemo(
    () =>
      new ApiClient({
        getAccessToken: (request) => authClient.getAccessToken(request),
      }),
    []
  );
  return <AuthenticatedApp apiClient={apiClient} authClient={authClient} />;
}

function AuthenticatedApp({
  apiClient,
  authClient,
}: {
  apiClient: ApiClient;
  authClient: AuthClient;
}) {
  const [authState, setAuthState] = useState<AuthState>(authClient.getState());
  const [libraryState, setLibraryState] = useState<LibraryState>({ status: "loading" });
  const [filters, setFilters] = useState<LibraryFilters>({});
  const [thumbnailUrls, setThumbnailUrls] = useState<Record<string, string>>({});
  const refreshLibrary = useCallback(
    () => loadLibrary(apiClient, filters, setLibraryState, setThumbnailUrls),
    [apiClient, filters]
  );

  useEffect(() => {
    const unsubscribe = authClient.subscribe(setAuthState);
    authClient.init().catch(() => {});
    return unsubscribe;
  }, [authClient]);

  useEffect(() => {
    if (authState.status !== "signed-in") {
      return;
    }
    refreshLibrary().catch(() => {});
  }, [authState.status, refreshLibrary]);

  if (authState.status !== "signed-in") {
    return <SignInPanel authClient={authClient} authState={authState} />;
  }
  return (
    <LibraryView
      filters={filters}
      state={libraryState}
      thumbnailUrls={thumbnailUrls}
      onCreateLink={(request) =>
        captureLinkItem(apiClient, request, setFilters, setLibraryState, setThumbnailUrls)
      }
      onCreateText={(request) =>
        captureTextItem(apiClient, request, setFilters, setLibraryState, setThumbnailUrls)
      }
      onDeleteItem={(itemId) =>
        deleteLibraryItem(apiClient, filters, itemId, setLibraryState, setThumbnailUrls)
      }
      onFiltersChange={setFilters}
      onSelectItem={(itemId) => {
        selectItem(apiClient, itemId, setLibraryState).catch(() => {});
      }}
      onCopyLink={copyLink}
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

async function loadLibrary(
  apiClient: ApiClient,
  filters: LibraryFilters,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void
) {
  setLibraryState({ status: "loading" });
  try {
    const data = await loadLibraryData(apiClient, filters);
    setLibraryState(readyLibraryState(data.items, data.tags));
    setThumbnailUrls(data.thumbnailUrls);
  } catch (error) {
    setLibraryState({
      status: "error",
      message: error instanceof Error ? error.message : "library failed to load",
    });
  }
}

async function captureTextItem(
  apiClient: ApiClient,
  request: CaptureTextRequest,
  setFilters: Dispatch<SetStateAction<LibraryFilters>>,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void
) {
  const outcome = await apiClient.captureText(request);
  await refreshUnfilteredWithDetail(
    apiClient,
    outcome.item,
    setFilters,
    setLibraryState,
    setThumbnailUrls
  );
  return outcome;
}

async function captureLinkItem(
  apiClient: ApiClient,
  request: CaptureLinkRequest,
  setFilters: Dispatch<SetStateAction<LibraryFilters>>,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void
) {
  const outcome = await apiClient.captureLink(request);
  await refreshUnfilteredWithDetail(
    apiClient,
    outcome.item,
    setFilters,
    setLibraryState,
    setThumbnailUrls
  );
  return outcome;
}

async function deleteLibraryItem(
  apiClient: ApiClient,
  filters: LibraryFilters,
  itemId: string,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void
) {
  await apiClient.deleteItem(itemId);
  const data = await loadLibraryData(apiClient, filters);
  setLibraryState(readyLibraryState(data.items, data.tags));
  setThumbnailUrls(data.thumbnailUrls);
}

async function refreshLibraryWithDetail(
  apiClient: ApiClient,
  filters: LibraryFilters,
  detail: LibraryItemDetail,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void
) {
  const data = await loadLibraryData(apiClient, filters);
  setLibraryState(selectedLibraryState(data.items, data.tags, detail));
  setThumbnailUrls(data.thumbnailUrls);
}

async function refreshUnfilteredWithDetail(
  apiClient: ApiClient,
  detail: LibraryItemDetail,
  setFilters: Dispatch<SetStateAction<LibraryFilters>>,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void
) {
  setFilters({});
  await refreshLibraryWithDetail(apiClient, {}, detail, setLibraryState, setThumbnailUrls);
}

async function loadLibraryData(apiClient: ApiClient, filters: LibraryFilters) {
  const [items, tags] = await Promise.all([
    apiClient.listItems(libraryFiltersToApiFilters(filters)),
    apiClient.listTags(),
  ]);
  return {
    items,
    tags,
    thumbnailUrls: await thumbnailUrlsForItems(apiClient, items),
  };
}

function selectedLibraryState(
  items: LibraryItemSummary[],
  tags: TagCorpusEntry[],
  detail: LibraryItemDetail
): LibraryState {
  return {
    status: "ready",
    items,
    tags,
    selectedItemId: detail.summary.id,
    selectedDetail: detail,
  };
}

async function selectItem(
  apiClient: ApiClient,
  itemId: string,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>
) {
  setLibraryState((state) => selectLibraryItem(state, itemId));
  const detail = await apiClient.getItem(itemId);
  setLibraryState((state) => replaceSelectedDetail(state, detail));
}

async function updateItemOrganization(
  apiClient: ApiClient,
  itemId: string,
  request: UpdateItemRequest,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>
) {
  const detail = await apiClient.updateItem(itemId, request);
  const tags = await apiClient.listTags();
  setLibraryState((state) => organizationState(state, detail, tags));
  return detail;
}

function organizationState(state: LibraryState, detail: LibraryItemDetail, tags: TagCorpusEntry[]) {
  const withDetail = replaceSelectedDetail(state, detail);
  const withSummary = applyUpdatedItemSummary(withDetail, detail.summary);
  return replaceTagCorpus(withSummary, tags);
}

async function renameLibraryTag(
  apiClient: ApiClient,
  filters: LibraryFilters,
  state: LibraryState,
  tagId: string,
  request: RenameTagRequest,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>
) {
  const tags = await apiClient.renameTag(tagId, request);
  await refreshAfterTagChange(apiClient, filters, selectedItemId(state), tags, setLibraryState);
  return tags;
}

async function mergeLibraryTags(
  apiClient: ApiClient,
  filters: LibraryFilters,
  state: LibraryState,
  sourceTagId: string,
  request: MergeTagsRequest,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>
) {
  const tags = await apiClient.mergeTags(sourceTagId, request);
  await refreshAfterTagChange(apiClient, filters, selectedItemId(state), tags, setLibraryState);
  return tags;
}

async function refreshAfterTagChange(
  apiClient: ApiClient,
  filters: LibraryFilters,
  selectedItemId: string | null,
  tags: TagCorpusEntry[],
  setLibraryState: Dispatch<SetStateAction<LibraryState>>
) {
  const [items, detail] = await Promise.all([
    apiClient.listItems(libraryFiltersToApiFilters(filters)),
    selectedItemId ? apiClient.getItem(selectedItemId).catch(() => null) : Promise.resolve(null),
  ]);
  setLibraryState((state) => tagRefreshState(state, items, tags, detail));
}

function tagRefreshState(
  state: LibraryState,
  items: LibraryItemSummary[],
  tags: TagCorpusEntry[],
  detail: LibraryItemDetail | null
): LibraryState {
  if (state.status !== "ready") {
    return state;
  }
  return {
    ...state,
    items,
    tags,
    selectedItemId: detail?.summary.id ?? state.selectedItemId ?? items[0]?.id ?? null,
    selectedDetail: detail,
  };
}

function selectedItemId(state: LibraryState) {
  return state.status === "ready" ? state.selectedItemId : null;
}

async function thumbnailUrlsForItems(
  apiClient: ApiClient,
  items: { id: string; thumbnail_s3_key: string | null }[]
) {
  const entries = await Promise.all(
    items
      .filter((item) => item.thumbnail_s3_key)
      .map(async (item) => [item.id, URL.createObjectURL(await apiClient.fetchThumbnail(item.id))])
  );
  return Object.fromEntries(entries);
}

function openSource(url: string) {
  window.open(url, "_blank", "noopener,noreferrer");
}

function copyLink(item: LibraryItemSummary) {
  copyCanonicalLink(item).catch(() => {});
}
