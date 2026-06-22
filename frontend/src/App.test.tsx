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
  it("renders_compact_item_controls_for_notes_tags_watch_and_inbox", async () => {
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

    expect(container.querySelector(".editable-title")).not.toBeNull();
    expect(container.querySelector(".notes-editor")).not.toBeNull();
    expect(container.querySelector(".tag-selector")).not.toBeNull();
    expect(findButtonByLabel(container, "Watch status: Unwatched")).not.toBeNull();
    expect(findButtonByLabel(container, "Inbox status: Unsorted")).not.toBeNull();
    expect(container.textContent).not.toContain("Save item");
    root.unmount();
    container.remove();
  });

  it("saves_item_organization_from_inline_controls", async () => {
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
            return updatedFixtureDetail(request);
          }}
          onRenameTag={renameTagNoop}
          onMergeTags={mergeTagsNoop}
        />
      );
    });

    await openFirstItem(container);
    await changeAndBlur(container.querySelector(".notes-editor"), "Filed after watching");
    await choosePopoverOption(container, "Watch status: Unwatched", "Watched");
    await choosePopoverOption(container, "Inbox status: Unsorted", "Organized");
    await selectTag(container, "Lerning");

    expect(updates).toEqual([
      { notes: "Filed after watching" },
      { watch_status: "watched" },
      { inbox_status: "organized" },
      { tags: ["Learning", "Lerning"] },
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

function setFieldValue(form: HTMLFormElement, name: string, value: string) {
  const field = form.elements.namedItem(name) as HTMLInputElement | HTMLTextAreaElement;
  field.value = value;
}

async function openFirstItem(container: HTMLElement) {
  const row = container.querySelector("tbody tr") as HTMLTableRowElement;
  await act(async () => {
    row.click();
  });
}

async function changeAndBlur(field: Element | null, value: string) {
  await act(async () => {
    const input = field as HTMLInputElement | HTMLTextAreaElement;
    setNativeValue(input, value);
    input.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    await Promise.resolve();
  });
}

async function choosePopoverOption(container: HTMLElement, triggerLabel: string, option: string) {
  await act(async () => {
    findButtonByLabel(container, triggerLabel).click();
    await Promise.resolve();
  });
  await act(async () => {
    findButton(container, option).click();
    await Promise.resolve();
  });
}

async function selectTag(container: HTMLElement, tag: string) {
  await act(async () => {
    container.querySelector<HTMLInputElement>('input[aria-label="Tags"]')?.click();
    await Promise.resolve();
  });
  await act(async () => {
    findButton(container, tag).click();
    await Promise.resolve();
  });
}

function setNativeValue(input: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const prototype =
    input instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(prototype, "value")?.set?.call(input, value);
}

function updatedFixtureDetail(request: UpdateItemRequest): LibraryItemDetail {
  const detail = libraryState.selectedDetail as LibraryItemDetail;
  return {
    ...detail,
    notes: request.notes ?? detail.notes,
    summary: {
      ...detail.summary,
      watch_status: request.watch_status ?? detail.summary.watch_status,
      inbox_status: request.inbox_status ?? detail.summary.inbox_status,
      tags: request.tags ? request.tags.map(fixtureTag) : detail.summary.tags,
    },
  };
}

function fixtureTag(tag: string) {
  return {
    id: tag.toLowerCase(),
    display_name: tag,
    normalized_name: tag.toLowerCase(),
  };
}

function findButton(container: HTMLElement, label: string) {
  const button = [...container.querySelectorAll("button")].find(
    (item) => item.textContent?.includes(label) ?? false
  );
  if (!button) {
    throw new Error(`Missing button: ${label}`);
  }
  return button;
}

function findButtonByLabel(container: HTMLElement, label: string) {
  const button = container.querySelector(`button[aria-label="${label}"]`);
  if (!button) {
    throw new Error(`Missing button: ${label}`);
  }
  return button as HTMLButtonElement;
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
