import type { Dispatch, SetStateAction } from "react";
import type { ApiClient } from "./api";
import { copyCanonicalLink } from "./itemActions";
import { libraryFiltersToApiFilters, type LibraryFilters } from "./libraryFilters";
import {
  applyUpdatedItemSummary,
  readyLibraryState,
  replaceSelectedDetail,
  replaceTagCorpus,
  selectLibraryItem,
  type LibraryState,
} from "./libraryState";
import { loadLibraryData } from "./loadLibraryData";
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

export async function loadLibrary(
  apiClient: ApiClient,
  filters: LibraryFilters,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void,
  setUpdatesCursor: (cursor: string) => void
) {
  setLibraryState((state) => (state.status === "ready" ? state : { status: "loading" }));
  try {
    const data = await loadLibraryData(apiClient, filters);
    setLibraryState(readyLibraryState(data.items, data.tags));
    setThumbnailUrls(data.thumbnailUrls);
    setUpdatesCursor(data.updatesCursor);
  } catch (error) {
    setLibraryState({
      status: "error",
      message: error instanceof Error ? error.message : "library failed to load",
    });
  }
}

export async function captureTextItem(
  apiClient: ApiClient,
  request: CaptureTextRequest,
  setFilters: Dispatch<SetStateAction<LibraryFilters>>,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void,
  setUpdatesCursor: (cursor: string) => void
) {
  const outcome = await apiClient.captureText(request);
  setFilters({});
  setLibraryState((state) => capturedDetailState(state, outcome.item));
  await refreshUnfilteredWithDetailIfAvailable(
    apiClient,
    outcome.item,
    setLibraryState,
    setThumbnailUrls,
    setUpdatesCursor
  );
  return outcome;
}

export async function captureLinkItem(
  apiClient: ApiClient,
  request: CaptureLinkRequest,
  setFilters: Dispatch<SetStateAction<LibraryFilters>>,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void,
  setUpdatesCursor: (cursor: string) => void
) {
  const outcome = await apiClient.captureLink(request);
  setFilters({});
  setLibraryState((state) => capturedDetailState(state, outcome.item));
  await refreshUnfilteredWithDetailIfAvailable(
    apiClient,
    outcome.item,
    setLibraryState,
    setThumbnailUrls,
    setUpdatesCursor
  );
  return outcome;
}

export async function deleteLibraryItem(
  apiClient: ApiClient,
  filters: LibraryFilters,
  itemId: string,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void,
  setUpdatesCursor: (cursor: string) => void
) {
  await apiClient.deleteItem(itemId);
  const data = await loadLibraryData(apiClient, filters);
  setLibraryState(readyLibraryState(data.items, data.tags));
  setThumbnailUrls(data.thumbnailUrls);
  setUpdatesCursor(data.updatesCursor);
}

export async function selectItem(
  apiClient: ApiClient,
  itemId: string,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>
) {
  setLibraryState((state) => selectLibraryItem(state, itemId));
  const detail = await apiClient.getItem(itemId);
  setLibraryState((state) => replaceSelectedDetail(state, detail));
}

export async function updateItemOrganization(
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

export async function renameLibraryTag(
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

export async function mergeLibraryTags(
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

export function openSource(url: string) {
  window.open(url, "_blank", "noopener,noreferrer");
}

export function copyLink(item: LibraryItemSummary) {
  copyCanonicalLink(item).catch(() => {});
}

async function refreshLibraryWithDetail(
  apiClient: ApiClient,
  filters: LibraryFilters,
  detail: LibraryItemDetail,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void,
  setUpdatesCursor: (cursor: string) => void
) {
  const data = await loadLibraryData(apiClient, filters);
  setLibraryState(selectedLibraryState(data.items, data.tags, detail));
  setThumbnailUrls(data.thumbnailUrls);
  setUpdatesCursor(data.updatesCursor);
}

async function refreshUnfilteredWithDetailIfAvailable(
  apiClient: ApiClient,
  detail: LibraryItemDetail,
  setLibraryState: Dispatch<SetStateAction<LibraryState>>,
  setThumbnailUrls: (urls: Record<string, string>) => void,
  setUpdatesCursor: (cursor: string) => void
) {
  try {
    await refreshLibraryWithDetail(
      apiClient,
      {},
      detail,
      setLibraryState,
      setThumbnailUrls,
      setUpdatesCursor
    );
  } catch {
    setLibraryState((state) => capturedDetailState(state, detail));
  }
}

function capturedDetailState(state: LibraryState, detail: LibraryItemDetail): LibraryState {
  if (state.status !== "ready") {
    return selectedLibraryState([detail.summary], [], detail);
  }
  return selectedLibraryState(capturedItems(state.items, detail.summary), state.tags, detail);
}

function capturedItems(items: LibraryItemSummary[], summary: LibraryItemSummary) {
  return [summary, ...items.filter((item) => item.id !== summary.id)];
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

function organizationState(state: LibraryState, detail: LibraryItemDetail, tags: TagCorpusEntry[]) {
  const withDetail = replaceSelectedDetail(state, detail);
  const withSummary = applyUpdatedItemSummary(withDetail, detail.summary);
  return replaceTagCorpus(withSummary, tags);
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
