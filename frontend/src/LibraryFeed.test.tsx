// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { LibraryFeed } from "./LibraryFeed";
import type { LibraryItemSummary } from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("LibraryFeed", () => {
  it("copies_existing_items_from_the_feed", async () => {
    const copied: LibraryItemSummary[] = [];
    const selected: string[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <LibraryFeed
          items={[textItem]}
          onCopyItem={(item) => copied.push(item)}
          onSelectItem={(itemId) => selected.push(itemId)}
          selectedItemId={null}
          thumbnailUrls={{}}
        />
      );
    });

    await clickButtonByLabel(container, "Copy text");

    expect(copied).toEqual([textItem]);
    expect(selected).toEqual([]);
    root.unmount();
    container.remove();
  });

  it("opens_detail_from_the_table_row", async () => {
    const selected: string[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <LibraryFeed
          items={[textItem]}
          onCopyItem={() => undefined}
          onSelectItem={(itemId) => selected.push(itemId)}
          selectedItemId={null}
          thumbnailUrls={{}}
        />
      );
    });

    await clickRow(container);

    expect(selected).toEqual(["item-1"]);
    root.unmount();
    container.remove();
  });

  it("renders_unknown_date_instead_of_crashing_on_bad_capture_dates", async () => {
    const item = { ...textItem, id: "bad-date", created_at: "" };
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <LibraryFeed
          items={[item]}
          onCopyItem={() => undefined}
          onSelectItem={() => undefined}
          selectedItemId={null}
          thumbnailUrls={{}}
        />
      );
    });

    expect(container.textContent).toContain("Unknown date");
    root.unmount();
    container.remove();
  });
});

async function clickButtonByLabel(container: HTMLElement, label: string) {
  const button = container.querySelector(`button[aria-label="${label}"]`) as HTMLButtonElement;
  await act(async () => {
    button.click();
  });
}

async function clickRow(container: HTMLElement) {
  const row = container.querySelector("tbody tr") as HTMLTableRowElement;
  await act(async () => {
    row.click();
  });
}

const textItem: LibraryItemSummary = {
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
};
