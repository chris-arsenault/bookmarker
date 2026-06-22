// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ItemDetail } from "./ItemDetail";
import type { LibraryItemDetail } from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ItemDetail delete confirmation", () => {
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

describe("ItemDetail link deletion state", () => {
  it("does_not_carry_a_stuck_deleting_state_between_link_items", async () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    let releaseDelete: (() => void) | null = null;
    const onDeleteItem = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          releaseDelete = resolve;
        })
    );

    await act(async () => {
      root.render(
        <ItemDetail
          availableTags={[]}
          detail={linkDetail("link-1")}
          onClose={() => undefined}
          onCopyLink={() => undefined}
          onDeleteItem={onDeleteItem}
          onOpenSource={() => undefined}
          onUpdateItem={async () => linkDetail("link-1")}
        />
      );
    });
    await act(async () => {
      clickButton(container, "Delete item");
      await Promise.resolve();
    });
    await act(async () => {
      clickButton(container, "Delete permanently");
      await Promise.resolve();
    });

    expect(findButton(container, "Deleting")?.disabled).toBe(true);

    await act(async () => {
      root.render(
        <ItemDetail
          availableTags={[]}
          detail={linkDetail("link-2")}
          onClose={() => undefined}
          onCopyLink={() => undefined}
          onDeleteItem={onDeleteItem}
          onOpenSource={() => undefined}
          onUpdateItem={async () => linkDetail("link-2")}
        />
      );
    });

    const deleteButton = findButton(container, "Delete item");
    expect(deleteButton?.disabled).toBe(false);
    releaseDelete?.();
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

    expect(container.querySelector("#detail-title")?.textContent).toBe("Terminal note");
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

describe("ItemDetail link heading", () => {
  it("shows_manual_title_fetched_title_and_url_without_a_preview_box", async () => {
    const detail = linkDetail("link-visible");
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <ItemDetail
          availableTags={[]}
          detail={detail}
          onClose={() => undefined}
          onCopyLink={() => undefined}
          onOpenSource={() => undefined}
          onUpdateItem={async () => detail}
        />
      );
    });

    expect(container.querySelector(".thumbnail")).toBeNull();
    expect(container.querySelector("#detail-title")?.textContent).toBe("Saved link");
    expect(container.textContent).toContain("Fetched title: Resolved metadata title");
    expect(container.textContent).toContain("https://example.com/link-visible");
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

function linkDetail(id: string): LibraryItemDetail {
  return {
    ...itemDetail,
    summary: {
      ...itemDetail.summary,
      id,
      item_kind: "url",
      url: {
        original_url: `https://example.com/${id}`,
        canonical_url: null,
        copy_url: `https://example.com/${id}`,
      },
      text: null,
      title: "Saved link",
      fetched_title: "Resolved metadata title",
      archive_status: "pending",
    },
  };
}

function clickButton(container: HTMLElement, label: string) {
  findButton(container, label)?.click();
}

function findButton(container: HTMLElement, label: string) {
  const button = [...container.querySelectorAll("button")].find(
    (element) => element.textContent === label
  ) as HTMLButtonElement;
  return button;
}
