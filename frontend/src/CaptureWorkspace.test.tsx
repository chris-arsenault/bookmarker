// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { CaptureWorkspace } from "./CaptureWorkspace";
import type {
  CaptureItemOutcome,
  CaptureLinkRequest,
  CaptureTextRequest,
  LibraryItemDetail,
} from "./types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("CaptureWorkspace text capture", () => {
  it("creates_text_items_from_a_notepad_style_body", async () => {
    const textRequests: CaptureTextRequest[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <CaptureWorkspace
          onCreateLink={createLinkNoop}
          onCreateText={async (request) => {
            textRequests.push(request);
            return captureOutcome("snippet-1");
          }}
          tags={tagCorpus}
        />
      );
    });

    await setTextarea(container, "copy this terminal output");
    await clickButton(container, "Research");
    await setInput(container, "Tags", "Shell, Research");
    await submitCapture(container);

    expect(container.textContent).not.toContain("Paste clipboard");
    expect(container.textContent).toContain("Saved to vault");
    expect(textRequests).toHaveLength(1);
    expect(textRequests[0]).toMatchObject({
      plain_text: "copy this terminal output",
      source_app: "Bookmarker",
      capture_method: "desktop_manual",
      tags: ["Research", "Shell"],
    });
    root.unmount();
    container.remove();
  });

  it("does_not_submit_empty_text_items", async () => {
    const textRequests: CaptureTextRequest[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <CaptureWorkspace
          onCreateLink={createLinkNoop}
          onCreateText={async (request) => {
            textRequests.push(request);
            return captureOutcome("snippet-1");
          }}
          tags={tagCorpus}
        />
      );
    });

    expect(saveButton(container).disabled).toBe(true);
    expect(textRequests).toEqual([]);
    root.unmount();
    container.remove();
  });
});

describe("CaptureWorkspace link capture", () => {
  it("creates_link_items_from_the_link_mode", async () => {
    const linkRequests: CaptureLinkRequest[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <CaptureWorkspace
          onCreateLink={async (request) => {
            linkRequests.push(request);
            return captureOutcome("link-1");
          }}
          onCreateText={createTextNoop}
          tags={tagCorpus}
        />
      );
    });

    await clickButton(container, "Link");
    await setInput(container, "URL", "https://example.com/watch");
    await clickButton(container, "Research");
    await submitCapture(container);

    expect(linkRequests).toEqual([
      expect.objectContaining({
        url: "https://example.com/watch",
        title: null,
        tags: ["Research"],
      }),
    ]);
    root.unmount();
    container.remove();
  });
});

const tagCorpus = [
  {
    id: "tag-1",
    display_name: "Research",
    normalized_name: "research",
    usage_count: 4,
  },
];

async function setTextarea(container: HTMLElement, value: string) {
  const textarea = container.querySelector("textarea") as HTMLTextAreaElement;
  await act(async () => {
    setNativeValue(textarea, value);
    textarea.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function setInput(container: HTMLElement, label: string, value: string) {
  const input = fieldAfterLabel(container, label) as HTMLInputElement;
  await act(async () => {
    setNativeValue(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

function fieldAfterLabel(container: HTMLElement, label: string) {
  return [...container.querySelectorAll("label")]
    .find((element) => element.textContent?.includes(label))
    ?.querySelector("input, textarea");
}

async function clickButton(container: HTMLElement, label: string) {
  const button = [...container.querySelectorAll("button")].find(
    (element) => element.textContent === label
  ) as HTMLButtonElement;
  await act(async () => {
    button.click();
  });
}

async function submitCapture(container: HTMLElement) {
  const form = container.querySelector("form.capture-form") as HTMLFormElement;
  await act(async () => {
    form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
  });
}

function saveButton(container: HTMLElement) {
  return [...container.querySelectorAll("button")].find(
    (button) => button.textContent === "Save item"
  ) as HTMLButtonElement;
}

function createTextNoop() {
  return Promise.resolve(captureOutcome("snippet-noop"));
}

function setNativeValue(field: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const prototype = Object.getPrototypeOf(field) as HTMLInputElement | HTMLTextAreaElement;
  const descriptor = Object.getOwnPropertyDescriptor(prototype, "value");
  descriptor?.set?.call(field, value);
}

function createLinkNoop() {
  return Promise.resolve(captureOutcome("link-noop"));
}

function captureOutcome(id: string): CaptureItemOutcome {
  return {
    created: true,
    item: itemDetail(id),
  };
}

function itemDetail(id: string): LibraryItemDetail {
  return {
    summary: {
      id,
      item_kind: "text_snippet",
      url: null,
      text: {
        plain_text: id,
        preview: id,
        content_hash: id,
        html: null,
        source_app: "Bookmarker",
        source_device: null,
        capture_method: "desktop_manual",
      },
      title: null,
      thumbnail_s3_key: null,
      author: null,
      platform: null,
      duration_seconds: null,
      archive_status: "not_applicable",
      watch_status: "unwatched",
      inbox_status: "unsorted",
      tags: [],
      created_at: "2026-06-15T00:00:00Z",
    },
    notes: "",
  };
}
