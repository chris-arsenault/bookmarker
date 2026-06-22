import { describe, expect, it } from "vitest";
import type { LibraryState } from "./libraryState";
import {
  ACTIVE_UPDATE_POLL_MS,
  IDLE_UPDATE_POLL_MS,
  applyLibraryUpdates,
  updatePollInterval,
} from "./libraryUpdates";
import type { LibraryItemDetail, LibraryItemSummary, LibraryUpdates } from "./types";

describe("library update polling", () => {
  it("polls_more_often_while_archive_work_is_pending", () => {
    expect(updatePollInterval([summary("pending", "pending")])).toBe(ACTIVE_UPDATE_POLL_MS);
    expect(updatePollInterval([summary("done", "succeeded")])).toBe(IDLE_UPDATE_POLL_MS);
  });

  it("applies_batched_item_updates_and_deletions_without_detail_fetches", () => {
    const selectedDetail = detail(summary("selected", "pending"));
    const state: LibraryState = {
      status: "ready",
      items: [summary("deleted", "succeeded"), selectedDetail.summary],
      tags: [],
      selectedItemId: "selected",
      selectedDetail,
    };

    const next = applyLibraryUpdates(state, {
      items: [{ ...selectedDetail.summary, archive_status: "succeeded", title: "Updated" }],
      deleted_item_ids: ["deleted"],
      tags: [tag("Research")],
      cursor: "2026-06-18T00:00:01Z",
    });

    expect(next.status).toBe("ready");
    if (next.status !== "ready") {
      return;
    }
    expect(next.items.map((item) => item.id)).toEqual(["selected"]);
    expect(next.items[0].title).toBe("Updated");
    expect(next.tags.map((entry) => entry.display_name)).toEqual(["Research"]);
    expect(next.selectedDetail?.summary.archive_status).toBe("succeeded");
  });

  it("moves_selection_when_the_selected_item_is_deleted", () => {
    const state: LibraryState = {
      status: "ready",
      items: [summary("kept", "succeeded"), summary("selected", "succeeded")],
      tags: [],
      selectedItemId: "selected",
      selectedDetail: detail(summary("selected", "succeeded")),
    };

    const next = applyLibraryUpdates(state, updates({ deleted_item_ids: ["selected"] }));

    expect(next.status).toBe("ready");
    if (next.status !== "ready") {
      return;
    }
    expect(next.selectedItemId).toBe("kept");
    expect(next.selectedDetail).toBeNull();
  });
});

function updates(overrides: Partial<LibraryUpdates>): LibraryUpdates {
  return {
    items: [],
    deleted_item_ids: [],
    tags: [],
    cursor: "2026-06-18T00:00:01Z",
    ...overrides,
  };
}

function detail(item: LibraryItemSummary): LibraryItemDetail {
  return {
    summary: item,
    notes: "",
  };
}

function tag(displayName: string) {
  return {
    id: displayName.toLowerCase(),
    display_name: displayName,
    normalized_name: displayName.toLowerCase(),
    usage_count: 1,
  };
}

function summary(id: string, archiveStatus: LibraryItemSummary["archive_status"]) {
  return {
    id,
    item_kind: "url",
    url: {
      original_url: `https://example.com/${id}`,
      canonical_url: null,
      copy_url: `https://example.com/${id}`,
    },
    text: null,
    title: id,
    thumbnail_s3_key: null,
    author: null,
    platform: "example",
    duration_seconds: null,
    archive_status: archiveStatus,
    watch_status: "unwatched",
    inbox_status: "unsorted",
    tags: [],
    created_at: "2026-06-18T00:00:00Z",
  } satisfies LibraryItemSummary;
}
