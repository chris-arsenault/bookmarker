// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { ItemOrganizer } from "./ItemOrganizer";
import type { LibraryItemDetail, TagCorpusEntry, UpdateItemRequest } from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ItemOrganizer inline editing", () => {
  it("saves_the_title_when_the_clicked_title_loses_focus", async () => {
    const updates: UpdateItemRequest[] = [];
    const view = renderOrganizer(async (_itemId, request) => {
      updates.push(request);
      return detailWith({ title: String(request.title) });
    });

    await editTitle(view.container, "Renamed note");

    expect(updates).toEqual([{ title: "Renamed note" }]);
    view.cleanup();
  });

  it("saves_notes_when_the_note_field_loses_focus", async () => {
    const updates: UpdateItemRequest[] = [];
    const view = renderOrganizer(async (_itemId, request) => {
      updates.push(request);
      return { ...itemDetail, notes: String(request.notes) };
    });

    await changeAndBlur(view.container.querySelector(".notes-editor"), "Filed after watching");

    expect(updates).toEqual([{ notes: "Filed after watching" }]);
    view.cleanup();
  });

  it("saves_watch_and_inbox_status_from_compact_popovers", async () => {
    const updates: UpdateItemRequest[] = [];
    const view = renderOrganizer(async (_itemId, request) => {
      updates.push(request);
      return detailWith(request);
    });

    await choosePopoverOption(view.container, "Watch status: Unwatched", "Watched");
    await choosePopoverOption(view.container, "Inbox status: Unsorted", "Organized");

    expect(updates).toEqual([{ watch_status: "watched" }, { inbox_status: "organized" }]);
    view.cleanup();
  });

  it("saves_tags_from_the_chip_selector", async () => {
    const updates: UpdateItemRequest[] = [];
    const view = renderOrganizer(async (_itemId, request) => {
      updates.push(request);
      return detailWith({ tags: tagDetails(request.tags ?? []) });
    }, availableTags);

    await selectTag(view.container, "Videos");

    expect(updates).toEqual([{ tags: ["Videos"] }]);
    view.cleanup();
  });

  it("shows_an_error_modal_when_the_api_does_not_return_the_requested_title", async () => {
    const view = renderOrganizer(async () => itemDetail);

    await editTitle(view.container, "Renamed note");

    expect(view.container.textContent).toContain("Title save did not persist");
    expect(view.container.querySelector(".error-modal")).not.toBeNull();
    view.cleanup();
  });
});

function renderOrganizer(
  onUpdateItem: (itemId: string, request: UpdateItemRequest) => Promise<LibraryItemDetail>,
  tags: TagCorpusEntry[] = []
) {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  act(() => {
    root.render(
      <ItemOrganizer
        availableTags={tags}
        density="default"
        detail={itemDetail}
        onUpdateItem={onUpdateItem}
      />
    );
  });
  return {
    container,
    cleanup: () => {
      root.unmount();
      container.remove();
    },
  };
}

async function editTitle(container: HTMLElement, title: string) {
  await act(async () => {
    findButton(container, "Terminal note").click();
    await Promise.resolve();
  });
  await changeAndBlur(container.querySelector('input[aria-label="Title"]'), title);
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

function detailWith(request: UpdateItemRequest): LibraryItemDetail {
  return {
    ...itemDetail,
    notes: request.notes ?? itemDetail.notes,
    summary: {
      ...itemDetail.summary,
      title: "title" in request ? String(request.title) : itemDetail.summary.title,
      watch_status: request.watch_status ?? itemDetail.summary.watch_status,
      inbox_status: request.inbox_status ?? itemDetail.summary.inbox_status,
      tags: request.tags ? tagDetails(request.tags) : itemDetail.summary.tags,
    },
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

function tagDetails(tags: string[]) {
  return tags.map((tag) => ({
    id: tag.toLowerCase(),
    display_name: tag,
    normalized_name: tag.toLowerCase(),
  }));
}

const availableTags: TagCorpusEntry[] = [
  { id: "tag-2", display_name: "Videos", normalized_name: "videos", usage_count: 3 },
  { id: "tag-1", display_name: "Learning", normalized_name: "learning", usage_count: 8 },
];

const itemDetail: LibraryItemDetail = {
  summary: {
    id: "item-1",
    item_kind: "text_snippet",
    url: null,
    image: null,
    text: {
      plain_text: "copy this terminal output",
      preview: "copy this terminal output",
      content_hash: "hash",
      html: null,
      source_app: "Terminal",
      source_device: null,
      capture_method: "desktop_clipboard",
    },
    title: "Terminal note",
    fetched_title: null,
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
  notes: "",
};
