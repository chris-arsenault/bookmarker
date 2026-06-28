import { config } from "./config";
import {
  authenticatedBlobRequest,
  authenticatedRequest,
  ApiClientError,
  type ApiClientOptions,
  type ApiRequestOptions,
} from "./apiCore";
import type {
  LibraryItemDetail,
  LibraryItemSummary,
  ImageAccessTarget,
  CaptureImageUploadOutcome,
  CaptureImageUploadRequest,
  CaptureLinkRequest,
  CaptureTextRequest,
  CaptureItemOutcome,
  LibraryUpdates,
  ListItemUpdatesRequest,
  ListItemsFilters,
  MergeTagsRequest,
  RenameTagRequest,
  TagCorpusEntry,
  UpdateItemRequest,
  WatchStatus,
} from "./types";

export { ApiClientError };
export type { AccessTokenRequest, ApiClientOptions } from "./apiCore";

export class ApiClient {
  private readonly baseUrl: string;
  private readonly options: ApiClientOptions;

  constructor(options: ApiClientOptions) {
    this.options = options;
    this.baseUrl = (options.baseUrl ?? config.apiBaseUrl).replace(/\/$/, "");
  }

  listItems(filters: ListItemsFilters = {}) {
    return this.request<LibraryItemSummary[]>(`/items${listItemsQuery(filters)}`);
  }

  listItemUpdates(request: ListItemUpdatesRequest = {}) {
    return this.request<LibraryUpdates>(`/items/updates${listItemUpdatesQuery(request)}`);
  }

  captureText(request: CaptureTextRequest) {
    return this.request<CaptureItemOutcome>("/items/text", {
      method: "POST",
      body: request,
    });
  }

  captureLink(request: CaptureLinkRequest) {
    return this.request<CaptureItemOutcome>("/items", {
      method: "POST",
      body: request,
    });
  }

  createImageUpload(request: CaptureImageUploadRequest) {
    return this.request<CaptureImageUploadOutcome>("/items/images/uploads", {
      method: "POST",
      body: request,
    });
  }

  completeImageUpload(itemId: string) {
    return this.request<LibraryItemDetail>(
      `/items/${encodeURIComponent(itemId)}/image-upload/complete`,
      { method: "POST" }
    );
  }

  getItem(itemId: string) {
    return this.request<LibraryItemDetail>(`/items/${encodeURIComponent(itemId)}`);
  }

  listTags() {
    return this.request<TagCorpusEntry[]>("/tags");
  }

  updateWatchStatus(itemId: string, watchStatus: WatchStatus) {
    return this.updateItem(itemId, { watch_status: watchStatus });
  }

  updateItem(itemId: string, request: UpdateItemRequest) {
    return this.request<LibraryItemDetail>(`/items/${encodeURIComponent(itemId)}`, {
      method: "PATCH",
      body: request,
    });
  }

  deleteItem(itemId: string) {
    return this.request<void>(`/items/${encodeURIComponent(itemId)}`, {
      method: "DELETE",
    });
  }

  renameTag(tagId: string, request: RenameTagRequest) {
    return this.request<TagCorpusEntry[]>(`/tags/${encodeURIComponent(tagId)}`, {
      method: "PATCH",
      body: request,
    });
  }

  mergeTags(sourceTagId: string, request: MergeTagsRequest) {
    return this.request<TagCorpusEntry[]>(`/tags/${encodeURIComponent(sourceTagId)}/merge`, {
      method: "POST",
      body: request,
    });
  }

  fetchThumbnail(itemId: string) {
    return authenticatedBlobRequest({
      baseUrl: this.baseUrl,
      clientOptions: this.options,
      path: `/items/${encodeURIComponent(itemId)}/thumbnail`,
    });
  }

  getImageAccess(itemId: string) {
    return this.request<ImageAccessTarget>(`/items/${encodeURIComponent(itemId)}/image`);
  }

  private request<T>(path: string, requestOptions: ApiRequestOptions = {}) {
    return authenticatedRequest<T>({
      baseUrl: this.baseUrl,
      clientOptions: this.options,
      path,
      requestOptions,
    });
  }
}

function listItemsQuery(filters: ListItemsFilters) {
  const params = new URLSearchParams();
  setListFilterParams(params, filters);
  return queryString(params);
}

function listItemUpdatesQuery(request: ListItemUpdatesRequest) {
  const params = new URLSearchParams();
  setParam(params, "since", request.since);
  if (request.limit !== undefined) {
    params.set("limit", request.limit.toString());
  }
  setListFilterParams(params, request);
  return queryString(params);
}

function setListFilterParams(params: URLSearchParams, filters: ListItemsFilters) {
  setParam(params, "platform", filters.platform);
  setParam(params, "tag", filters.tag);
  setParam(params, "created_from", filters.createdFrom);
  setParam(params, "created_to", filters.createdTo);
  setParam(params, "archive_status", filters.archiveStatus);
  setParam(params, "watch_status", filters.watchStatus);
  setParam(params, "inbox_status", filters.inboxStatus);
  setParam(params, "q", filters.q);
}

function setParam(params: URLSearchParams, name: string, value: string | undefined) {
  if (value && value.trim().length > 0) {
    params.set(name, value);
  }
}

function queryString(params: URLSearchParams) {
  const query = params.toString();
  return query.length > 0 ? `?${query}` : "";
}
