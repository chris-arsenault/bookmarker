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

  it("copyCanonicalLink writes image filenames", async () => {
    const writes: string[] = [];

    await copyCanonicalLink(imageItem(), { writeText: async (value) => writes.push(value) });

    expect(writes).toEqual(["phone.jpg"]);
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
    image: null,
  } satisfies LibraryItemSummary;
}

function textItem() {
  return {
    ...baseItem(),
    item_kind: "text_snippet",
    url: null,
    image: null,
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

function imageItem() {
  return {
    ...baseItem(),
    item_kind: "image",
    url: null,
    text: null,
    image: {
      s3_key: "images/item-1/original",
      content_type: "image/jpeg",
      original_filename: "phone.jpg",
      byte_size: 2048,
      upload_status: "uploaded",
      source_app: "Android share",
      source_device: "android",
      capture_method: "android_share",
    },
    archive_status: "succeeded",
  } satisfies LibraryItemSummary;
}

function baseItem() {
  return {
    id: "item-1",
    title: null,
    fetched_title: null,
    image: null,
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
