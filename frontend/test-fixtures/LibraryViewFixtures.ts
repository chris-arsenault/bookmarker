import type { LibraryFilters } from "../src/libraryFilters";
import type { LibraryState } from "../src/libraryState";
import type { LibraryItemDetail } from "../src/types";

type ReadyLibraryState = Extract<LibraryState, { status: "ready" }>;

export const emptyFilters: LibraryFilters = {};

export const libraryState: ReadyLibraryState = {
  status: "ready",
  selectedItemId: "item-1",
  selectedDetail: {
    summary: {
      id: "item-1",
      item_kind: "url",
      url: {
        original_url: "https://example.com/original",
        canonical_url: "https://example.com/watch",
        copy_url: "https://example.com/watch",
      },
      text: null,
      title: "Saved video",
      thumbnail_s3_key: "snapshots/item/thumbnail.jpg",
      author: "Creator",
      platform: "YouTube",
      duration_seconds: 120,
      archive_status: "failed",
      watch_status: "unwatched",
      inbox_status: "unsorted",
      tags: [{ id: "tag-1", display_name: "Learning", normalized_name: "learning" }],
      created_at: "2026-06-15T00:00:00Z",
    },
    notes: "watch for API shape",
  },
  items: [
    {
      id: "item-1",
      item_kind: "url",
      url: {
        original_url: "https://example.com/original",
        canonical_url: "https://example.com/watch",
        copy_url: "https://example.com/watch",
      },
      text: null,
      title: "Saved video",
      thumbnail_s3_key: "snapshots/item/thumbnail.jpg",
      author: "Creator",
      platform: "YouTube",
      duration_seconds: 120,
      archive_status: "failed",
      watch_status: "unwatched",
      inbox_status: "unsorted",
      tags: [{ id: "tag-1", display_name: "Learning", normalized_name: "learning" }],
      created_at: "2026-06-15T00:00:00Z",
    },
    {
      id: "item-2",
      item_kind: "text_snippet",
      url: null,
      text: {
        plain_text: "copy this terminal output",
        preview: "copy this terminal output",
        content_hash: "hash",
        html: null,
        source_app: "Terminal",
        source_device: null,
        capture_method: "desktop_clipboard",
      },
      title: null,
      thumbnail_s3_key: null,
      author: null,
      platform: null,
      duration_seconds: null,
      archive_status: "not_applicable",
      watch_status: "unwatched",
      inbox_status: "unsorted",
      tags: [],
      created_at: "2026-06-15T00:00:00Z",
    },
  ],
  tags: [
    { id: "tag-1", display_name: "Learning", normalized_name: "learning", usage_count: 1 },
    { id: "tag-2", display_name: "Lerning", normalized_name: "lerning", usage_count: 1 },
  ],
};

export const emptyTagState: ReadyLibraryState = {
  status: "ready",
  selectedItemId: "item-empty",
  selectedDetail: {
    summary: {
      id: "item-empty",
      item_kind: "url",
      url: {
        original_url: "https://example.com/empty",
        canonical_url: null,
        copy_url: "https://example.com/empty",
      },
      text: null,
      title: "Untagged video",
      thumbnail_s3_key: null,
      author: null,
      platform: "YouTube",
      duration_seconds: null,
      archive_status: "pending",
      watch_status: "unwatched",
      inbox_status: "unsorted",
      tags: [],
      created_at: "2026-06-15T00:00:00Z",
    },
    notes: "",
  },
  items: [
    {
      id: "item-empty",
      item_kind: "url",
      url: {
        original_url: "https://example.com/empty",
        canonical_url: null,
        copy_url: "https://example.com/empty",
      },
      text: null,
      title: "Untagged video",
      thumbnail_s3_key: null,
      author: null,
      platform: "YouTube",
      duration_seconds: null,
      archive_status: "pending",
      watch_status: "unwatched",
      inbox_status: "unsorted",
      tags: [],
      created_at: "2026-06-15T00:00:00Z",
    },
  ],
  tags: [],
};

export function updateItemNoop() {
  return Promise.resolve(libraryState.selectedDetail as LibraryItemDetail);
}

export function renameTagNoop() {
  return Promise.resolve(libraryState.tags);
}

export function mergeTagsNoop() {
  return Promise.resolve(libraryState.tags);
}
