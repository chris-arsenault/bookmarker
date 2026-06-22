// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { ItemOrganizer } from "./ItemOrganizer";
import type { LibraryItemDetail, UpdateItemRequest } from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ItemOrganizer title editing", () => {
  it("sends_the_edited_title_with_the_item_update", async () => {
    const updates: UpdateItemRequest[] = [];
    const view = renderOrganizer(async (_itemId, request) => {
      updates.push(request);
      return detailWithTitle(String(request.title));
    });

    await editTitleAndSave(view.container, "Renamed note");

    expect(updates).toContainEqual(expect.objectContaining({ title: "Renamed note" }));
    view.cleanup();
  });

  it("shows_an_error_when_the_api_does_not_return_the_requested_title", async () => {
    const view = renderOrganizer(async () => itemDetail);

    await editTitleAndSave(view.container, "Renamed note");

    expect(view.container.textContent).toContain("Title save did not persist");
    view.cleanup();
  });
});

function renderOrganizer(
  onUpdateItem: (itemId: string, request: UpdateItemRequest) => Promise<LibraryItemDetail>
) {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  act(() => {
    root.render(
      <ItemOrganizer
        availableTags={[]}
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

async function editTitleAndSave(container: HTMLElement, title: string) {
  const titleInput = container.querySelector<HTMLInputElement>('input[name="title"]');
  await act(async () => {
    titleInput!.value = title;
    findButton(container, "Save item")?.click();
    await Promise.resolve();
  });
}

function detailWithTitle(title: string): LibraryItemDetail {
  return {
    ...itemDetail,
    summary: {
      ...itemDetail.summary,
      title,
    },
  };
}

function findButton(container: HTMLElement, label: string) {
  return [...container.querySelectorAll("button")].find((button) => button.textContent === label);
}

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
