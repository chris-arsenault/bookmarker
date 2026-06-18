// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ItemDetail } from "./ItemDetail";
import type { LibraryItemDetail } from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ItemDetail deletion", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("confirms_and_deletes_the_selected_item", async () => {
    const deletedItems: string[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    const confirm = vi.fn(() => true);
    Object.defineProperty(window, "confirm", {
      configurable: true,
      value: confirm,
    });

    await act(async () => {
      root.render(
        <ItemDetail
          availableTags={[]}
          detail={itemDetail}
          onCopyLink={() => undefined}
          onDeleteItem={async (itemId) => {
            deletedItems.push(itemId);
          }}
          onOpenSource={() => undefined}
          onUpdateItem={async () => itemDetail}
        />
      );
    });

    await act(async () => {
      clickButton(container, "Delete item");
      await Promise.resolve();
    });

    expect(confirm).toHaveBeenCalledWith("Delete this item?");
    expect(deletedItems).toEqual(["item-1"]);
    root.unmount();
    container.remove();
  });
});

const itemDetail: LibraryItemDetail = {
  summary: {
    id: "item-1",
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
  notes: "",
};

function clickButton(container: HTMLElement, label: string) {
  const button = [...container.querySelectorAll("button")].find(
    (element) => element.textContent === label
  ) as HTMLButtonElement;
  button.click();
}
