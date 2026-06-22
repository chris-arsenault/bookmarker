import { describe, expect, it } from "vitest";
import { copyCanonicalLink } from "./itemActions";
import type { LibraryItemSummary } from "./types";

describe("item actions", () => {
  it("copyCanonicalLink writes the canonical copy_url", async () => {
    const writes: string[] = [];

    await copyCanonicalLink(urlItem(), { writeText: async (value) => writes.push(value) });

    expect(writes).toEqual(["https://example.com/canonical"]);
  });

  it("copyCanonicalLink writes text snippets", async () => {
    const writes: string[] = [];

    await copyCanonicalLink(textItem(), { writeText: async (value) => writes.push(value) });

    expect(writes).toEqual(["snippet body"]);
  });
});

function urlItem() {
  return {
    ...baseItem(),
    item_kind: "url",
    url: {
      original_url: "https://example.com/original",
      canonical_url: "https://example.com/canonical",
      copy_url: "https://example.com/canonical",
    },
    text: null,
  } satisfies LibraryItemSummary;
}

function textItem() {
  return {
    ...baseItem(),
    item_kind: "text_snippet",
    url: null,
    text: {
      plain_text: "snippet body",
      preview: "snippet body",
      content_hash: "hash",
      html: null,
      source_app: null,
      source_device: null,
      capture_method: "desktop_clipboard",
    },
    archive_status: "not_applicable",
  } satisfies LibraryItemSummary;
}

function baseItem() {
  return {
    id: "item-1",
    title: null,
    fetched_title: null,
    thumbnail_s3_key: null,
    author: null,
    platform: null,
    duration_seconds: null,
    archive_status: "pending" as const,
    watch_status: "unwatched" as const,
    inbox_status: "unsorted" as const,
    tags: [],
    created_at: "2026-06-15T00:00:00Z",
  };
}
