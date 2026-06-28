// @vitest-environment happy-dom

import { act, useCallback, useMemo } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ImageAccessProvider } from "./ImageAccessProvider";
import { ItemDetail } from "./ItemDetail";
import type { LibraryActions } from "./LibraryActionsContext";
import { LibraryActionsProvider } from "./LibraryActionsProvider";
import type {
  ArchiveStatus,
  ImageAccessTarget,
  ImageUploadStatus,
  LibraryItemDetail,
} from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const noTags: [] = [];
const closeNoop = () => undefined;

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ItemDetail images", () => {
  it("loads_uploaded_images_into_the_detail_modal", async () => {
    const loaded: string[] = [];
    const detail = imageDetail();
    const container = mountedContainer();
    const root = createRoot(container);

    await act(async () => {
      root.render(imageDetailView(detail, loaded));
      await Promise.resolve();
    });

    expect(loaded).toEqual(["image-1"]);
    expect(container.querySelector("#detail-title")?.textContent).toBe("Phone transfer");
    expect((container.querySelector(".image-detail-preview") as HTMLImageElement)?.src).toBe(
      "https://download.example.test/images/image-1/original"
    );
    expect(container.querySelector(".image-download")?.getAttribute("download")).toBe("phone.jpg");
    expect(container.querySelector(".image-download")?.getAttribute("href")).toBe(
      "https://download.example.test/images/image-1/original?download=phone.jpg"
    );
    cleanup(root, container);
  });

  it("does_not_reload_image_access_when_the_parent_rerenders", async () => {
    const loaded: string[] = [];
    const detail = imageDetail();
    const container = mountedContainer();
    const root = createRoot(container);

    await act(async () => {
      root.render(imageDetailView(detail, loaded));
      await Promise.resolve();
    });
    await act(async () => {
      root.render(imageDetailView(detail, loaded));
      await Promise.resolve();
    });

    expect(loaded).toEqual(["image-1"]);
    cleanup(root, container);
  });

  it("does_not_fetch_image_access_while_upload_is_pending", async () => {
    const loaded: string[] = [];
    const detail = imageDetail("pending", "pending");
    const container = mountedContainer();
    const root = createRoot(container);

    await act(async () => {
      root.render(imageDetailView(detail, loaded));
      await Promise.resolve();
    });

    expect(loaded).toEqual([]);
    expect(container.textContent).toContain("Image upload pending");
    cleanup(root, container);
  });
});

function imageDetailView(detail: LibraryItemDetail, loaded: string[]) {
  return <ImageDetailHarness detail={detail} loaded={loaded} />;
}

function ImageDetailHarness({ detail, loaded }: { detail: LibraryItemDetail; loaded: string[] }) {
  const actions = useMemo(() => detailActions(detail), [detail]);
  const loadImageAccess = useCallback(
    async (itemId: string) => {
      loaded.push(itemId);
      return imageAccessTarget();
    },
    [loaded]
  );
  return (
    <LibraryActionsProvider actions={actions}>
      <ImageAccessProvider loadImageAccess={loadImageAccess}>
        <ItemDetail availableTags={noTags} detail={detail} onClose={closeNoop} />
      </ImageAccessProvider>
    </LibraryActionsProvider>
  );
}

function detailActions(detail: LibraryItemDetail): LibraryActions {
  return {
    changeFilters: () => undefined,
    closeDetail: () => undefined,
    copyItem: () => undefined,
    createLink: async () => ({ created: true, item: detail }),
    createText: async () => ({ created: true, item: detail }),
    deleteItem: async () => undefined,
    mergeTags: async () => [],
    openSource: () => undefined,
    renameTag: async () => [],
    selectItem: () => undefined,
    updateItem: async () => detail,
  };
}

function imageDetail(
  uploadStatus: ImageUploadStatus = "uploaded",
  archiveStatus: ArchiveStatus = "succeeded"
): LibraryItemDetail {
  return {
    summary: {
      id: "image-1",
      item_kind: "image",
      url: null,
      text: null,
      image: {
        s3_key: "images/image-1/original",
        content_type: "image/jpeg",
        original_filename: "phone.jpg",
        byte_size: 2048,
        upload_status: uploadStatus,
        source_app: "Android share",
        source_device: "android",
        capture_method: "android_share",
      },
      title: "Phone transfer",
      fetched_title: null,
      thumbnail_s3_key: null,
      author: null,
      platform: null,
      duration_seconds: null,
      archive_status: archiveStatus,
      watch_status: "unwatched",
      inbox_status: "unsorted",
      tags: [],
      created_at: "2026-06-15T00:00:00Z",
    },
    notes: "",
  };
}

function imageAccessTarget(): ImageAccessTarget {
  return {
    view_url: "https://download.example.test/images/image-1/original",
    download_url: "https://download.example.test/images/image-1/original?download=phone.jpg",
    content_type: "image/jpeg",
    download_name: "phone.jpg",
    expires_in_seconds: 600,
  };
}

function mountedContainer() {
  const container = document.createElement("div");
  document.body.append(container);
  return container;
}

function cleanup(root: ReturnType<typeof createRoot>, container: HTMLElement) {
  root.unmount();
  container.remove();
}
