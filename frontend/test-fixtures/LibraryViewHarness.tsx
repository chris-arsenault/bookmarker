import { useMemo, useState, type Dispatch, type SetStateAction } from "react";
import { ImageAccessProvider } from "../src/ImageAccessProvider";
import type { LibraryActions } from "../src/LibraryActionsContext";
import { LibraryActionsProvider } from "../src/LibraryActionsProvider";
import { LibraryView } from "../src/LibraryView";
import type {
  CaptureItemOutcome,
  CaptureLinkRequest,
  CaptureTextRequest,
  LibraryItemDetail,
  UpdateItemRequest,
} from "../src/types";
import type { LibrarySnapshot } from "../src/useLibraryController";
import {
  emptyFilters,
  libraryState,
  mergeTagsNoop,
  renameTagNoop,
  updateItemNoop,
} from "./LibraryViewFixtures";

type LibraryViewOptions = {
  actions: Partial<LibraryActions>;
  state: LibrarySnapshot["state"];
  thumbnailUrls: LibrarySnapshot["thumbnailUrls"];
};

const noActionOverrides: Partial<LibraryActions> = {};
const noThumbnails: LibrarySnapshot["thumbnailUrls"] = {};

export function libraryView(options: Partial<LibraryViewOptions> = {}) {
  return (
    <LibraryViewTestHarness
      actions={options.actions ?? noActionOverrides}
      initialState={options.state ?? libraryState}
      thumbnailUrls={options.thumbnailUrls ?? noThumbnails}
    />
  );
}

function LibraryViewTestHarness({
  actions: actionOverrides,
  initialState,
  thumbnailUrls,
}: {
  actions: Partial<LibraryActions>;
  initialState: LibrarySnapshot["state"];
  thumbnailUrls: LibrarySnapshot["thumbnailUrls"];
}) {
  const [state, setState] = useState(initialState);
  const actions = useMemo(() => testLibraryActions(actionOverrides, setState), [actionOverrides]);
  const snapshot = useMemo<LibrarySnapshot>(
    () => ({
      filters: emptyFilters,
      state,
      thumbnailUrls,
    }),
    [state, thumbnailUrls]
  );
  return (
    <LibraryActionsProvider actions={actions}>
      <ImageAccessProvider loadImageAccess={imageAccessNoop}>
        <LibraryView snapshot={snapshot} />
      </ImageAccessProvider>
    </LibraryActionsProvider>
  );
}

function testLibraryActions(
  overrides: Partial<LibraryActions>,
  setState: Dispatch<SetStateAction<LibrarySnapshot["state"]>>
): LibraryActions {
  const { updateItem: customUpdateItem = updateItemNoop, ...actionOverrides } = overrides;
  const updateItem = async (itemId: string, request: UpdateItemRequest) => {
    const detail = await customUpdateItem(itemId, request);
    setState((current) => replaceHarnessDetail(current, detail));
    return detail;
  };
  return {
    changeFilters: () => undefined,
    closeDetail: () => undefined,
    copyItem: () => undefined,
    createLink: createLinkNoop,
    createText: createTextNoop,
    deleteItem: async () => undefined,
    mergeTags: mergeTagsNoop,
    openSource: () => undefined,
    renameTag: renameTagNoop,
    selectItem: () => undefined,
    ...actionOverrides,
    updateItem,
  };
}

function replaceHarnessDetail(
  state: LibrarySnapshot["state"],
  detail: LibraryItemDetail
): LibrarySnapshot["state"] {
  if (state.status !== "ready") {
    return state;
  }
  return {
    ...state,
    items: state.items.map((item) => (item.id === detail.summary.id ? detail.summary : item)),
    selectedDetail: detail,
    selectedItemId: detail.summary.id,
  };
}

async function createTextNoop(request: CaptureTextRequest): Promise<CaptureItemOutcome> {
  return { created: true, item: textDetail(request.plain_text) };
}

async function createLinkNoop(request: CaptureLinkRequest): Promise<CaptureItemOutcome> {
  return { created: true, item: linkDetail(request.url) };
}

async function imageAccessNoop() {
  return {
    content_type: "image/jpeg",
    download_name: "image.jpg",
    download_url: "https://example.test/download",
    expires_in_seconds: 600,
    view_url: "https://example.test/view",
  };
}

function textDetail(text: string): LibraryItemDetail {
  return {
    summary: {
      ...libraryState.items[1],
      text: {
        plain_text: text,
        preview: text,
        content_hash: text,
        html: null,
        source_app: "Bookmarker",
        source_device: null,
        capture_method: "desktop_manual",
      },
    },
    notes: "",
  };
}

function linkDetail(url: string): LibraryItemDetail {
  return {
    summary: {
      ...libraryState.items[0],
      id: "link-harness",
      url: {
        original_url: url,
        canonical_url: url,
        copy_url: url,
      },
    },
    notes: "",
  };
}
