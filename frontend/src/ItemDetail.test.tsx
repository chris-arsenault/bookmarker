// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ItemDetail } from "./ItemDetail";
import type { LibraryItemDetail } from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ItemDetail delete confirmation", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("confirms_and_deletes_the_selected_item_without_a_browser_popup", async () => {
    const deletedItems: string[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    const confirm = vi.fn(() => {
      throw new Error("window.confirm should not be used");
    });
    Object.defineProperty(window, "confirm", {
      configurable: true,
      value: confirm,
    });

    await act(async () => {
      root.render(
        <ItemDetail
          availableTags={[]}
          detail={itemDetail}
          onClose={() => undefined}
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

    expect(confirm).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Delete this item?");
    expect(deletedItems).toEqual([]);

    await act(async () => {
      clickButton(container, "Delete permanently");
      await Promise.resolve();
    });

    expect(deletedItems).toEqual(["item-1"]);
    root.unmount();
    container.remove();
  });
});

describe("ItemDetail delete cancellation", () => {
  it("cancels_the_custom_delete_confirmation", async () => {
    const deletedItems: string[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <ItemDetail
          availableTags={[]}
          detail={itemDetail}
          onClose={() => undefined}
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
    });
    await act(async () => {
      clickButton(container, "Cancel");
    });

    expect(container.textContent).not.toContain("Delete this item?");
    expect(deletedItems).toEqual([]);
    root.unmount();
    container.remove();
  });
});

describe("ItemDetail text snippets", () => {
  it("renders_saved_text_as_the_primary_content_area", async () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <ItemDetail
          availableTags={[]}
          detail={longSnippetDetail}
          onClose={() => undefined}
          onCopyLink={() => undefined}
          onOpenSource={() => undefined}
          onUpdateItem={async () => longSnippetDetail}
        />
      );
    });

    expect(container.querySelector("#detail-title")?.textContent).toBe("Saved text");
    expect(container.querySelector(".thumbnail")).toBeNull();
    expect(container.querySelector(".status-badge")).toBeNull();
    expect(container.querySelector(".snippet-body-primary")?.textContent).toContain(
      "first copied line"
    );
    expect(container.querySelector(".markdown-snippet strong")?.textContent).toBe(
      "first copied line"
    );
    expect(container.querySelector('textarea[name="notes"]')?.getAttribute("rows")).toBe("2");
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

const longSnippetDetail: LibraryItemDetail = {
  ...itemDetail,
  summary: {
    ...itemDetail.summary,
    text: {
      plain_text: [
        "**first copied line**",
        "- second copied line with enough text to wrap inside the detail modal",
        "- third copied line",
      ].join("\n"),
      preview: "first copied line",
      content_hash: "hash-long",
      html: null,
      source_app: "Terminal",
      source_device: null,
      capture_method: "desktop_clipboard",
    },
  },
  notes: "small note",
};

function clickButton(container: HTMLElement, label: string) {
  const button = [...container.querySelectorAll("button")].find(
    (element) => element.textContent === label
  ) as HTMLButtonElement;
  button.click();
}
