// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { LibraryView } from "./LibraryView";
import {
  emptyFilters,
  emptyTagState,
  libraryState,
  mergeTagsNoop,
  renameTagNoop,
  updateItemNoop,
} from "../test-fixtures/LibraryViewFixtures";
import type {
  LibraryItemDetail,
  MergeTagsRequest,
  RenameTagRequest,
  UpdateItemRequest,
} from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("LibraryView browsing", () => {
  it("renders_authenticated_library_table_with_status_and_collapsed_search", () => {
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
    expect(html).toContain("Search and filters");
    expect(html).toContain('aria-label="Copy link"');
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
  it("renders_item_organizer_for_notes_tags_watch_and_inbox", async () => {
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
          onRenameTag={renameTagNoop}
          onMergeTags={mergeTagsNoop}
        />
      );
    });

    await openFirstItem(container);

    expect(container.textContent).toContain("Save item");
    expect(container.textContent).toContain("Notes");
    expect(container.textContent).toContain("Unwatched");
    expect(container.textContent).toContain("Organized");
    expect(container.textContent).toContain("New tag");
    root.unmount();
    container.remove();
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

    await openFirstItem(container);
    const form = container.querySelector("form.item-organizer") as HTMLFormElement;
    setFieldValue(form, "notes", "Filed after watching");
    setFieldValue(form, "new_tag", "Videos");
    checkRadio(container, "watch_status", "watched");
    checkRadio(container, "inbox_status", "organized");

    await act(async () => {
      form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });

    expect(updates).toEqual([expectedOrganizationUpdate()]);
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

function expectedOrganizationUpdate(): UpdateItemRequest {
  return {
    title: "Saved video",
    watch_status: "watched",
    inbox_status: "organized",
    notes: "Filed after watching",
    tags: ["Learning", "Videos"],
  };
}

function setFieldValue(form: HTMLFormElement, name: string, value: string) {
  const field = form.elements.namedItem(name) as HTMLInputElement | HTMLTextAreaElement;
  field.value = value;
}

function checkRadio(container: HTMLElement, name: string, value: string) {
  const field = container.querySelector(`input[name="${name}"][value="${value}"]`);
  (field as HTMLInputElement).checked = true;
}

async function openFirstItem(container: HTMLElement) {
  const row = container.querySelector("tbody tr") as HTMLTableRowElement;
  await act(async () => {
    row.click();
  });
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
