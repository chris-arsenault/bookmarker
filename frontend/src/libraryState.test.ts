import { describe, expect, it } from "vitest";
import {
  applyUpdatedItemSummary,
  clearSelectedItem,
  createLibraryViewModel,
  replaceSelectedDetail,
  replaceTagCorpus,
  type LibraryState,
} from "./libraryState";
import type { LibraryItemSummary, TagCorpusEntry } from "./types";

describe("library state", () => {
  it("createLibraryViewModel exposes selected empty pending and failed states", () => {
    const pending = item("pending-item", "pending");
    const failed = item("failed-item", "failed");
    const state: LibraryState = {
      status: "ready",
      items: [pending, failed],
      tags: [tag("Learning")],
      selectedItemId: failed.id,
      selectedDetail: { summary: failed, notes: "source was removed" },
    };

    const viewModel = createLibraryViewModel(state);

    expect(viewModel.isEmpty).toBe(false);
    expect(viewModel.hasPendingArchive).toBe(true);
    expect(viewModel.hasFailedArchive).toBe(true);
    expect(viewModel.selectedItem?.id).toBe(failed.id);
    expect(viewModel.selectedDetail?.notes).toBe("source was removed");
  });

  it("library_state_replaces_selected_detail_after_organization", () => {
    const original = item("saved-item", "pending");
    const updated = {
      ...original,
      watch_status: "watched",
      inbox_status: "organized",
      tags: [itemTag("Learning")],
    };
    let state: LibraryState = {
      status: "ready",
      items: [original],
      tags: [tag("Learning")],
      selectedItemId: original.id,
      selectedDetail: { summary: original, notes: "" },
    };

    state = replaceSelectedDetail(state, { summary: updated, notes: "Filed" });
    state = applyUpdatedItemSummary(state, updated);
    state = replaceTagCorpus(state, [tag("Research")]);

    expect(state.status).toBe("ready");
    if (state.status !== "ready") {
      return;
    }
    expect(state.selectedDetail?.notes).toBe("Filed");
    expect(state.selectedDetail?.summary.inbox_status).toBe("organized");
    expect(state.items[0].watch_status).toBe("watched");
    expect(state.tags[0].display_name).toBe("Research");
  });

  it("library_state_clears_selected_detail_after_modal_close", () => {
    const selected = item("saved-item", "succeeded");
    const state = clearSelectedItem({
      status: "ready",
      items: [selected],
      tags: [tag("Learning")],
      selectedItemId: selected.id,
      selectedDetail: { summary: selected, notes: "open" },
    });

    expect(state.status).toBe("ready");
    if (state.status !== "ready") {
      return;
    }
    expect(state.selectedItemId).toBeNull();
    expect(state.selectedDetail).toBeNull();
    expect(createLibraryViewModel(state).selectedItem).toBeNull();
  });
});

function item(id: string, archiveStatus: LibraryItemSummary["archive_status"]) {
  return {
    id,
    item_kind: "url",
    url: {
      original_url: `https://example.com/${id}`,
      canonical_url: null,
      copy_url: `https://example.com/${id}`,
    },
    text: null,
    image: null,
    title: id,
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
  } satisfies LibraryItemSummary;
}

function tag(displayName: string) {
  return {
    id: "tag-1",
    display_name: displayName,
    normalized_name: displayName.toLowerCase(),
    usage_count: 1,
  } satisfies TagCorpusEntry;
}

function itemTag(displayName: string) {
  return {
    id: "tag-1",
    display_name: displayName,
    normalized_name: displayName.toLowerCase(),
  };
}
