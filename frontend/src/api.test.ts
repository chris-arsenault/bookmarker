import { describe, expect, it } from "vitest";
import { ApiClient } from "./api";
import type { FetchLike } from "./apiCore";

describe("ApiClient requests", () => {
  it("api_client_serializes_library_filters_and_auth", async () => {
    const calls: CapturedRequest[] = [];
    const client = new ApiClient({
      baseUrl: "https://api.example.test/",
      getAccessToken: async () => "access-token",
      fetchImpl: captureFetch(calls, jsonResponse([])),
    });

    await client.listItems({
      platform: "YouTube",
      tag: "Learning",
      createdFrom: "2026-06-01T00:00:00Z",
      createdTo: "2026-06-15T00:00:00Z",
      archiveStatus: "succeeded",
      watchStatus: "unwatched",
      q: "pipeline notes",
    });

    expect(calls[0].url).toBe(
      "https://api.example.test/items?platform=YouTube&tag=Learning&created_from=2026-06-01T00%3A00%3A00Z&created_to=2026-06-15T00%3A00%3A00Z&archive_status=succeeded&watch_status=unwatched&q=pipeline+notes"
    );
    expect(calls[0].authorization).toBe("Bearer access-token");
  });

  it("api_client_serializes_inbox_filter", async () => {
    const calls: CapturedRequest[] = [];
    const client = new ApiClient({
      baseUrl: "https://api.example.test/",
      getAccessToken: async () => "access-token",
      fetchImpl: captureFetch(calls, jsonResponse([])),
    });

    await client.listItems({ inboxStatus: "organized" });

    expect(calls[0].url).toBe("https://api.example.test/items?inbox_status=organized");
  });
});

describe("ApiClient update polling", () => {
  it("api_client_serializes_batched_update_filters", async () => {
    const calls: CapturedRequest[] = [];
    const client = new ApiClient({
      baseUrl: "https://api.example.test/",
      getAccessToken: async () => "access-token",
      fetchImpl: captureFetch(
        calls,
        jsonResponse({ items: [], deleted_item_ids: [], tags: [], cursor: "2026-06-18T00:00:01Z" })
      ),
    });

    await client.listItemUpdates({
      since: "2026-06-18T00:00:00Z",
      limit: 25,
      platform: "YouTube",
      tag: "Learning",
      archiveStatus: "pending",
      inboxStatus: "unsorted",
      q: "metadata refresh",
    });

    expect(calls[0].url).toBe(
      "https://api.example.test/items/updates?since=2026-06-18T00%3A00%3A00Z&limit=25&platform=YouTube&tag=Learning&archive_status=pending&inbox_status=unsorted&q=metadata+refresh"
    );
  });
});

describe("ApiClient organization", () => {
  it("api_client_updates_item_organization", async () => {
    const calls: CapturedRequest[] = [];
    const client = new ApiClient({
      baseUrl: "https://api.example.test",
      getAccessToken: async () => "access-token",
      fetchImpl: sequenceFetch(calls, [
        jsonResponse({ summary: itemSummary("item-1"), notes: "Filed" }),
        jsonResponse([]),
        jsonResponse([]),
      ]),
    });

    await client.updateItem("item-1", {
      watch_status: "watched",
      inbox_status: "organized",
      notes: "Filed",
      tags: ["Learning"],
    });
    await client.renameTag("tag-1", { display_name: "Research" });
    await client.mergeTags("tag-2", { target_tag_id: "tag-1" });

    expect(calls.map((call) => call.method)).toEqual(["PATCH", "PATCH", "POST"]);
    expect(calls.map((call) => call.url)).toEqual([
      "https://api.example.test/items/item-1",
      "https://api.example.test/tags/tag-1",
      "https://api.example.test/tags/tag-2/merge",
    ]);
    expect(JSON.parse(calls[0].body ?? "{}")).toEqual({
      watch_status: "watched",
      inbox_status: "organized",
      notes: "Filed",
      tags: ["Learning"],
    });
    expect(JSON.parse(calls[1].body ?? "{}")).toEqual({ display_name: "Research" });
    expect(JSON.parse(calls[2].body ?? "{}")).toEqual({ target_tag_id: "tag-1" });
  });
});

describe("ApiClient text capture", () => {
  it("api_client_captures_text_snippets", async () => {
    const calls: CapturedRequest[] = [];
    const client = new ApiClient({
      baseUrl: "https://api.example.test",
      getAccessToken: async () => "access-token",
      fetchImpl: captureFetch(
        calls,
        jsonResponse({ item: itemDetail("snippet-1"), created: true })
      ),
    });

    await client.captureText({
      plain_text: "keep this",
      html: null,
      source_app: "Terminal",
      source_device: null,
      capture_method: "desktop_clipboard",
      tags: ["Shell"],
      client_capture_id: "desktop-1",
    });

    expect(calls[0].url).toBe("https://api.example.test/items/text");
    expect(calls[0].method).toBe("POST");
    expect(JSON.parse(calls[0].body ?? "{}")).toEqual({
      plain_text: "keep this",
      html: null,
      source_app: "Terminal",
      source_device: null,
      capture_method: "desktop_clipboard",
      tags: ["Shell"],
      client_capture_id: "desktop-1",
    });
  });
});

describe("ApiClient item capture", () => {
  it("api_client_captures_links_and_deletes_items", async () => {
    const calls: CapturedRequest[] = [];
    const client = new ApiClient({
      baseUrl: "https://api.example.test",
      getAccessToken: async () => "access-token",
      fetchImpl: sequenceFetch(calls, [
        jsonResponse({ item: itemDetail("link-1"), created: true }),
        new Response(null, { status: 204 }),
      ]),
    });

    await client.captureLink({
      url: "https://example.com/watch",
      title: "Manual link title",
      tags: ["Research"],
      client_capture_id: "manual-link-1",
    });
    await client.deleteItem("link-1");

    expect(calls.map((call) => call.method)).toEqual(["POST", "DELETE"]);
    expect(calls.map((call) => call.url)).toEqual([
      "https://api.example.test/items",
      "https://api.example.test/items/link-1",
    ]);
    expect(JSON.parse(calls[0].body ?? "{}")).toEqual({
      url: "https://example.com/watch",
      title: "Manual link title",
      tags: ["Research"],
      client_capture_id: "manual-link-1",
    });
  });
});

describe("ApiClient auth", () => {
  it("api_client_refreshes_once_on_unauthorized", async () => {
    const calls: CapturedRequest[] = [];
    const tokens: string[] = [];
    const client = new ApiClient({
      baseUrl: "https://api.example.test",
      getAccessToken: async (request) => {
        tokens.push(request?.forceRefresh ? "refresh" : "initial");
        return request?.forceRefresh ? "fresh-token" : "expired-token";
      },
      fetchImpl: sequenceFetch(calls, [
        jsonResponse({ code: "unauthorized" }, 401),
        jsonResponse([]),
      ]),
    });

    await client.listTags();

    expect(tokens).toEqual(["initial", "refresh"]);
    expect(calls.map((call) => call.authorization)).toEqual([
      "Bearer expired-token",
      "Bearer fresh-token",
    ]);
  });
});

describe("ApiClient errors", () => {
  it("api_client_uses_a_non_empty_message_for_empty_error_payloads", async () => {
    const client = new ApiClient({
      baseUrl: "https://api.example.test",
      getAccessToken: async () => "access-token",
      fetchImpl: async () => jsonResponse({ message: "" }, 500),
    });

    await expect(client.listTags()).rejects.toThrow("HTTP 500");
  });
});

type CapturedRequest = {
  url: string;
  authorization: string | null;
  method: string | undefined;
  body: string | undefined;
};

function captureFetch(calls: CapturedRequest[], response: Response): FetchLike {
  return async (input, init) => {
    calls.push(captureRequest(input, init));
    return response;
  };
}

function sequenceFetch(calls: CapturedRequest[], responses: Response[]): FetchLike {
  return async (input, init) => {
    calls.push(captureRequest(input, init));
    const response = responses.shift();
    if (!response) {
      throw new Error("no response queued");
    }
    return response;
  };
}

function captureRequest(input: RequestInfo | URL, init?: RequestInit): CapturedRequest {
  const headers = new Headers(init?.headers);
  return {
    url: input.toString(),
    authorization: headers.get("authorization"),
    method: init?.method,
    body: typeof init?.body === "string" ? init.body : undefined,
  };
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function itemSummary(id: string) {
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
    platform: null,
    duration_seconds: null,
    archive_status: "pending",
    watch_status: "watched",
    inbox_status: "organized",
    tags: [],
    created_at: "2026-06-15T00:00:00Z",
  };
}

function itemDetail(id: string) {
  return {
    summary: itemSummary(id),
    notes: "",
  };
}
