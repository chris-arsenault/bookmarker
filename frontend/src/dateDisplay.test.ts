import { describe, expect, it } from "vitest";
import { formatDate } from "./dateDisplay";

describe("formatDate", () => {
  it("formats_iso_dates", () => {
    expect(formatDate("2026-06-15T00:00:00Z")).toBe("June 15, 2026");
  });

  it("formats_postgres_style_timestamps", () => {
    expect(formatDate("2026-06-15 13:45:00.123456 +00:00:00")).toBe("June 15, 2026");
  });

  it("formats_unix_second_timestamps", () => {
    expect(formatDate(Date.UTC(2026, 5, 15) / 1000)).toBe("June 15, 2026");
  });

  it("formats_time_serde_tuple_timestamps", () => {
    expect(formatDate([Date.UTC(2026, 5, 15) / 1000, 0])).toBe("June 15, 2026");
  });

  it("formats_calendar_tuple_timestamps", () => {
    expect(formatDate([2026, 6, 15, 13, 45, 0, 123_000_000])).toBe("June 15, 2026");
  });

  it("still_reports_unknown_for_invalid_dates", () => {
    expect(formatDate("not a date")).toBe("Unknown date");
  });
});
