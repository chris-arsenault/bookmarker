// @vitest-environment happy-dom

import { describe, expect, it, vi } from "vitest";
import type { ApiClient } from "./api";
import { mergePreviewUrls, previewUrlsForItems } from "./itemPreviewUrls";
import type { LibraryItemSummary } from "./types";

describe("item preview URLs", () => {
  it("loads_archived_thumbnails_without_bulk_fetching_uploaded_image_bytes", async () => {
    const originalCreateObjectUrl = URL.createObjectURL;
    URL.createObjectURL = vi.fn(() => "blob:snapshot-preview");
    const fetchThumbnail = vi.fn(async () => new Blob(["thumbnail"], { type: "image/jpeg" }));
    const getImageAccess = vi.fn();

    const urls = await previewUrlsForItems(
      { fetchThumbnail, getImageAccess } as unknown as ApiClient,
      [urlItemWithThumbnail(), uploadedImageItem()]
    );

    expect(urls).toEqual({ "url-1": "blob:snapshot-preview" });
    expect(fetchThumbnail).toHaveBeenCalledWith("url-1");
    expect(getImageAccess).not.toHaveBeenCalled();
    URL.createObjectURL = originalCreateObjectUrl;
  });

  it("removes_stale_preview_urls_when_items_no_longer_have_archived_thumbnails", () => {
    const next = mergePreviewUrls(
      { "image-1": "blob:old-image", "url-1": "blob:old-url" },
      [uploadedImageItem(), urlItemWithoutThumbnail()],
      {}
    );

    expect(next).toEqual({});
  });
});

function urlItemWithThumbnail(): LibraryItemSummary {
  return {
    ...baseItem("url-1"),
    item_kind: "url",
    url: {
      original_url: "https://example.com/watch",
      canonical_url: null,
      copy_url: "https://example.com/watch",
    },
    thumbnail_s3_key: "snapshots/url-1/thumbnail.jpg",
    archive_status: "succeeded",
  };
}

function urlItemWithoutThumbnail(): LibraryItemSummary {
  return {
    ...urlItemWithThumbnail(),
    thumbnail_s3_key: null,
  };
}

function uploadedImageItem(): LibraryItemSummary {
  return {
    ...baseItem("image-1"),
    item_kind: "image",
    image: {
      s3_key: "images/image-1/original",
      content_type: "image/jpeg",
      original_filename: "phone.jpg",
      byte_size: 2048,
      upload_status: "uploaded",
      source_app: "Android share",
      source_device: "android",
      capture_method: "android_share",
    },
    title: "Phone transfer",
    archive_status: "succeeded",
  };
}

function baseItem(id: string): LibraryItemSummary {
  return {
    id,
    item_kind: "text_snippet",
    url: null,
    text: null,
    image: null,
    title: null,
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
  };
}
