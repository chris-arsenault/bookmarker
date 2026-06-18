// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { LibraryView } from "./LibraryView";
import type { LibraryState } from "./libraryState";
import type { LibraryFilters } from "./libraryFilters";
import type {
  LibraryItemDetail,
  MergeTagsRequest,
  RenameTagRequest,
  UpdateItemRequest,
} from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("LibraryView browsing", () => {
  it("renders_authenticated_library_feed_with_detail_and_status", () => {
    const html = renderToString(
      <LibraryView
        state={libraryState}
        filters={emptyFilters}
        thumbnailUrls={{ "item-1": "/items/item-1/thumbnail" }}
        onFiltersChange={() => undefined}
        onSelectItem={() => undefined}
        onCopyLink={() => undefined}
        onOpenSource={() => undefined}
        onUpdateItem={updateItemNoop}
        onRenameTag={renameTagNoop}
        onMergeTags={mergeTagsNoop}
      />
    );

    expect(html).toContain("Saved video");
    expect(html).toContain("copy this terminal output");
    expect(html).toContain("failed");
    expect(html).toContain("Learning");
    expect(html).toContain("June 15, 2026");
    expect(html).toContain("watch for API shape");
    expect(html).toContain("Open source");
  });

  it("renders_filter_controls_for_platform_tag_date_archive_watch_and_text", () => {
    const html = renderToString(
      <LibraryView
        state={libraryState}
        filters={emptyFilters}
        thumbnailUrls={{ "item-1": "/items/item-1/thumbnail" }}
        onFiltersChange={() => undefined}
        onSelectItem={() => undefined}
        onCopyLink={() => undefined}
        onOpenSource={() => undefined}
        onUpdateItem={updateItemNoop}
        onRenameTag={renameTagNoop}
        onMergeTags={mergeTagsNoop}
      />
    );

    expect(html).toContain("Platform");
    expect(html).toContain("Tag");
    expect(html).toContain("Created from");
    expect(html).toContain("Archive");
    expect(html).toContain("Watch status");
    expect(html).toContain("Search");
    expect(html).toContain("Copy link");
  });
});

describe("LibraryView organizer", () => {
  it("renders_item_organizer_for_notes_tags_watch_and_inbox", () => {
    const html = renderToString(
      <LibraryView
        state={libraryState}
        filters={emptyFilters}
        thumbnailUrls={{ "item-1": "/items/item-1/thumbnail" }}
        onFiltersChange={() => undefined}
        onSelectItem={() => undefined}
        onCopyLink={() => undefined}
        onOpenSource={() => undefined}
        onUpdateItem={updateItemNoop}
        onRenameTag={renameTagNoop}
        onMergeTags={mergeTagsNoop}
      />
    );

    expect(html).toContain("Save item");
    expect(html).toContain("Notes");
    expect(html).toContain("Unwatched");
    expect(html).toContain("Organized");
    expect(html).toContain("New tag");
  });

  it("submits_item_organization_update", async () => {
    const updates: UpdateItemRequest[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <LibraryView
          state={libraryState}
          filters={emptyFilters}
          thumbnailUrls={{ "item-1": "/items/item-1/thumbnail" }}
          onFiltersChange={() => undefined}
          onSelectItem={() => undefined}
          onCopyLink={() => undefined}
          onOpenSource={() => undefined}
          onUpdateItem={async (_itemId, request) => {
            updates.push(request);
            return libraryState.selectedDetail as LibraryItemDetail;
          }}
          onRenameTag={renameTagNoop}
          onMergeTags={mergeTagsNoop}
        />
      );
    });

    const form = container.querySelector("form.item-organizer") as HTMLFormElement;
    setFieldValue(form, "notes", "Filed after watching");
    setFieldValue(form, "new_tag", "Videos");
    checkRadio(container, "watch_status", "watched");
    checkRadio(container, "inbox_status", "organized");

    await act(async () => {
      form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });

    expect(updates).toEqual([
      {
        watch_status: "watched",
        inbox_status: "organized",
        notes: "Filed after watching",
        tags: ["Learning", "Videos"],
      },
    ]);
    root.unmount();
    container.remove();
  });
});

describe("LibraryView tag management", () => {
  it("renders_tag_manager_for_rename_and_merge", async () => {
    const renameCalls: [string, RenameTagRequest][] = [];
    const mergeCalls: [string, MergeTagsRequest][] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <LibraryView
          state={libraryState}
          filters={emptyFilters}
          thumbnailUrls={{ "item-1": "/items/item-1/thumbnail" }}
          onFiltersChange={() => undefined}
          onSelectItem={() => undefined}
          onCopyLink={() => undefined}
          onOpenSource={() => undefined}
          onUpdateItem={updateItemNoop}
          onRenameTag={async (tagId, request) => {
            renameCalls.push([tagId, request]);
            return libraryState.tags;
          }}
          onMergeTags={async (tagId, request) => {
            mergeCalls.push([tagId, request]);
            return libraryState.tags;
          }}
        />
      );
    });

    expect(container.textContent).toContain("Tag management");
    expect(container.textContent).toContain("Rename");
    expect(container.textContent).toContain("Merge");
    submitRename(container, "tag-2", "Learning");
    submitMerge(container, "tag-2", "tag-1");

    expect(renameCalls).toEqual([["tag-2", { display_name: "Learning" }]]);
    expect(mergeCalls).toEqual([["tag-2", { target_tag_id: "tag-1" }]]);
    root.unmount();
    container.remove();
  });

  it("renders_empty_tag_corpus_without_starter_tags", () => {
    const html = renderToString(
      <LibraryView
        state={emptyTagState}
        filters={emptyFilters}
        thumbnailUrls={{}}
        onFiltersChange={() => undefined}
        onSelectItem={() => undefined}
        onCopyLink={() => undefined}
        onOpenSource={() => undefined}
        onUpdateItem={updateItemNoop}
        onRenameTag={renameTagNoop}
        onMergeTags={mergeTagsNoop}
      />
    );

    expect(html).toContain("No tags yet");
    expect(html).not.toContain("Starter");
  });
});

const emptyFilters: LibraryFilters = {};

const libraryState: LibraryState = {
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
      tags: [
        {
          id: "tag-1",
          display_name: "Learning",
          normalized_name: "learning",
        },
      ],
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
      tags: [
        {
          id: "tag-1",
          display_name: "Learning",
          normalized_name: "learning",
        },
      ],
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
    {
      id: "tag-1",
      display_name: "Learning",
      normalized_name: "learning",
      usage_count: 1,
    },
    {
      id: "tag-2",
      display_name: "Lerning",
      normalized_name: "lerning",
      usage_count: 1,
    },
  ],
};

const emptyTagState: LibraryState = {
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

function updateItemNoop() {
  return Promise.resolve(libraryState.selectedDetail as LibraryItemDetail);
}

function renameTagNoop() {
  return Promise.resolve(libraryState.tags);
}

function mergeTagsNoop() {
  return Promise.resolve(libraryState.tags);
}

function setFieldValue(form: HTMLFormElement, name: string, value: string) {
  const field = form.elements.namedItem(name) as HTMLInputElement | HTMLTextAreaElement;
  field.value = value;
}

function checkRadio(container: HTMLElement, name: string, value: string) {
  const field = container.querySelector(`input[name="${name}"][value="${value}"]`);
  (field as HTMLInputElement).checked = true;
}

function submitRename(container: HTMLElement, tagId: string, value: string) {
  const form = container.querySelector(
    `form[data-tag-action="rename"][data-tag-id="${tagId}"]`
  ) as HTMLFormElement;
  setFieldValue(form, "display_name", value);
  form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
}

function submitMerge(container: HTMLElement, sourceTagId: string, targetTagId: string) {
  const form = container.querySelector(
    `form[data-tag-action="merge"][data-tag-id="${sourceTagId}"]`
  ) as HTMLFormElement;
  setFieldValue(form, "target_tag_id", targetTagId);
  form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
}
