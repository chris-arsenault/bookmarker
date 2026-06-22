// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { QuickTextCapture } from "./QuickTextCapture";
import type { CaptureItemOutcome, CaptureLinkRequest, CaptureTextRequest } from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("QuickTextCapture text mode", () => {
  it("saves_plain_text_without_tags", async () => {
    const requests: CaptureTextRequest[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <QuickTextCapture
          tags={[]}
          onCreateLink={createLinkNoop}
          onCreateText={async (request) => {
            requests.push(request);
            return { created: true, item: textItemDetail(request) };
          }}
        />
      );
    });

    await setInput(container, "quick-title", "Shell note");
    await setTextarea(container, "remember this shell output");
    await submitForm(container);

    expect(requests).toHaveLength(1);
    expect(requests[0]).toMatchObject({
      plain_text: "remember this shell output",
      title: "Shell note",
      tags: [],
      capture_method: "desktop_manual",
    });
    expect(container.textContent).toContain("Saved");
    root.unmount();
    container.remove();
  });

  it("saves_plain_text_with_selected_and_free_text_tags", async () => {
    const requests: CaptureTextRequest[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <QuickTextCapture
          tags={tagCorpus}
          onCreateLink={createLinkNoop}
          onCreateText={async (request) => {
            requests.push(request);
            return { created: true, item: textItemDetail(request) };
          }}
        />
      );
    });

    await setTextarea(container, "remember this shell output");
    await clickButton(container, "Work");
    await setInput(container, "quick-tags", "Later, work");
    await submitForm(container);

    expect(requests[0].tags).toEqual(["Work", "Later"]);
    root.unmount();
    container.remove();
  });
});

describe("QuickTextCapture link mode", () => {
  it("saves_links_from_the_same_new_item_box", async () => {
    const textRequests: CaptureTextRequest[] = [];
    const linkRequests: CaptureLinkRequest[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <QuickTextCapture
          tags={tagCorpus}
          onCreateLink={async (request) => {
            linkRequests.push(request);
            return { created: true, item: linkItemDetail(request) };
          }}
          onCreateText={async (request) => {
            textRequests.push(request);
            return { created: true, item: textItemDetail(request) };
          }}
        />
      );
    });

    await chooseMode(container, "link");
    await setInput(container, "quick-title", "Example article");
    await setInput(container, "quick-url", " https://example.com/read ");
    await clickButton(container, "Work");
    await submitForm(container);

    expect(textRequests).toEqual([]);
    expect(linkRequests).toHaveLength(1);
    expect(linkRequests[0]).toMatchObject({
      url: "https://example.com/read",
      title: "Example article",
      tags: ["Work"],
    });
    expect(container.textContent).toContain("Saved");
    root.unmount();
    container.remove();
  });
});

describe("QuickTextCapture errors", () => {
  it("shows_save_failures_in_a_copyable_modal", async () => {
    const writes: string[] = [];
    mockClipboard(writes);
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <QuickTextCapture
          tags={[]}
          onCreateLink={createLinkNoop}
          onCreateText={async () => {
            throw new Error("backend rejected text");
          }}
        />
      );
    });

    await setTextarea(container, "remember this shell output");
    await submitForm(container);
    await clickButton(container, "Copy");

    expect(container.textContent).toContain("The item was not saved.");
    expect(container.textContent).toContain("Save failed: backend rejected text");
    expect(writes).toEqual(["Save failed: backend rejected text"]);
    expect(container.textContent).toContain("Copied");
    root.unmount();
    container.remove();
  });

  it("shows_a_real_failure_message_when_error_message_is_empty", async () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <QuickTextCapture
          tags={[]}
          onCreateLink={createLinkNoop}
          onCreateText={async () => {
            throw new Error("");
          }}
        />
      );
    });

    await setTextarea(container, "remember this shell output");
    await submitForm(container);

    expect(container.textContent).toContain("Save failed: unknown error");
    root.unmount();
    container.remove();
  });
});

async function setTextarea(container: HTMLElement, value: string) {
  const textarea = container.querySelector("textarea") as HTMLTextAreaElement;
  await act(async () => {
    setNativeValue(textarea, value);
    textarea.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function setInput(container: HTMLElement, name: string, value: string) {
  const input = container.querySelector(`input[name="${name}"]`) as HTMLInputElement;
  await act(async () => {
    setNativeValue(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function chooseMode(container: HTMLElement, mode: "text" | "link") {
  const input = container.querySelector(
    `input[name="quick-capture-mode"][value="${mode}"]`
  ) as HTMLInputElement;
  await act(async () => {
    input.click();
  });
}

async function submitForm(container: HTMLElement) {
  const form = container.querySelector("form") as HTMLFormElement;
  await act(async () => {
    form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
  });
}

async function clickButton(container: HTMLElement, label: string) {
  const button = [...container.querySelectorAll("button")].find(
    (candidate) => candidate.textContent === label
  ) as HTMLButtonElement;
  await act(async () => {
    button.click();
  });
}

function mockClipboard(writes: string[]) {
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: {
      writeText: async (value: string) => {
        writes.push(value);
      },
    },
  });
}

function setNativeValue(element: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(element.constructor.prototype, "value")?.set;
  setter?.call(element, value);
}

async function createLinkNoop(request: CaptureLinkRequest): Promise<CaptureItemOutcome> {
  return { created: true, item: linkItemDetail(request) };
}

const tagCorpus = [
  {
    id: "tag-1",
    display_name: "Work",
    normalized_name: "work",
    usage_count: 4,
  },
  {
    id: "tag-2",
    display_name: "Reading",
    normalized_name: "reading",
    usage_count: 2,
  },
];

function textItemDetail(request: CaptureTextRequest) {
  return {
    summary: {
      id: "item-1",
      item_kind: "text_snippet" as const,
      url: null,
      text: {
        plain_text: request.plain_text,
        preview: request.plain_text,
        content_hash: "hash",
        html: null,
        source_app: "Bookmarker",
        source_device: null,
        capture_method: "desktop_manual",
      },
      title: request.title,
      fetched_title: null,
      thumbnail_s3_key: null,
      author: null,
      platform: null,
      duration_seconds: null,
      archive_status: "not_applicable" as const,
      watch_status: "unwatched" as const,
      inbox_status: "unsorted" as const,
      tags: [],
      created_at: "2026-06-15T00:00:00Z",
    },
    notes: "",
  };
}

function linkItemDetail(request: CaptureLinkRequest) {
  return {
    summary: {
      id: "item-link",
      item_kind: "url" as const,
      url: {
        original_url: request.url,
        canonical_url: request.url,
        copy_url: request.url,
      },
      text: null,
      title: request.title,
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
    },
    notes: "",
  };
}
