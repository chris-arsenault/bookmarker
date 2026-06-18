import { describe, expect, it } from "vitest";
import { libraryFiltersToApiFilters } from "./libraryFilters";

describe("library filters", () => {
  it("library_filters_serialize_platform_tag_date_status_and_text", () => {
    expect(
      libraryFiltersToApiFilters({
        platform: " YouTube ",
        tag: " learning ",
        createdFrom: "2026-06-01T00:00:00Z",
        createdTo: "2026-06-15T00:00:00Z",
        archiveStatus: "failed",
        watchStatus: "watched",
        inboxStatus: "organized",
        q: " pipeline notes ",
      })
    ).toEqual({
      platform: "YouTube",
      tag: "learning",
      createdFrom: "2026-06-01T00:00:00Z",
      createdTo: "2026-06-15T00:00:00Z",
      archiveStatus: "failed",
      watchStatus: "watched",
      inboxStatus: "organized",
      q: "pipeline notes",
    });
  });
});
