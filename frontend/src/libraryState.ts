import type { LibraryItemDetail, LibraryItemSummary, TagCorpusEntry } from "./types";

export type LibraryState =
  | { status: "loading" }
  | { status: "signed-out" }
  | { status: "error"; message: string }
  | {
      status: "ready";
      items: LibraryItemSummary[];
      tags: TagCorpusEntry[];
      selectedItemId: string | null;
      selectedDetail: LibraryItemDetail | null;
    };

export type LibraryViewModel = {
  status: LibraryState["status"];
  items: LibraryItemSummary[];
  tags: TagCorpusEntry[];
  isEmpty: boolean;
  selectedItem: LibraryItemSummary | null;
  selectedDetail: LibraryItemDetail | null;
  hasPendingArchive: boolean;
  hasFailedArchive: boolean;
  errorMessage: string | null;
};

export function createLibraryViewModel(state: LibraryState): LibraryViewModel {
  if (state.status !== "ready") {
    return inactiveViewModel(state);
  }
  const selectedItem = selectedSummary(state.items, state.selectedItemId);
  return {
    status: state.status,
    items: state.items,
    tags: state.tags,
    isEmpty: state.items.length === 0,
    selectedItem,
    selectedDetail: state.selectedDetail,
    hasPendingArchive: hasArchiveStatus(state.items, "pending"),
    hasFailedArchive: hasArchiveStatus(state.items, "failed"),
    errorMessage: null,
  };
}

export function readyLibraryState(
  items: LibraryItemSummary[],
  tags: TagCorpusEntry[]
): LibraryState {
  return {
    status: "ready",
    items,
    tags,
    selectedItemId: items[0]?.id ?? null,
    selectedDetail: null,
  };
}

export function selectLibraryItem(state: LibraryState, itemId: string): LibraryState {
  if (state.status !== "ready") {
    return state;
  }
  return {
    ...state,
    selectedItemId: itemId,
  };
}

export function clearSelectedItem(state: LibraryState): LibraryState {
  if (state.status !== "ready") {
    return state;
  }
  return {
    ...state,
    selectedItemId: null,
    selectedDetail: null,
  };
}

export function replaceSelectedDetail(
  state: LibraryState,
  detail: LibraryItemDetail
): LibraryState {
  if (state.status !== "ready") {
    return state;
  }
  return {
    ...state,
    selectedItemId: detail.summary.id,
    selectedDetail: detail,
  };
}

export function applyUpdatedItemSummary(
  state: LibraryState,
  summary: LibraryItemSummary
): LibraryState {
  if (state.status !== "ready") {
    return state;
  }
  return {
    ...state,
    items: state.items.map((item) => (item.id === summary.id ? summary : item)),
    selectedDetail: updatedSelectedDetail(state.selectedDetail, summary),
  };
}

export function replaceTagCorpus(state: LibraryState, tags: TagCorpusEntry[]): LibraryState {
  if (state.status !== "ready") {
    return state;
  }
  return {
    ...state,
    tags,
  };
}

function inactiveViewModel(state: Exclude<LibraryState, { status: "ready" }>): LibraryViewModel {
  return {
    status: state.status,
    items: [],
    tags: [],
    isEmpty: true,
    selectedItem: null,
    selectedDetail: null,
    hasPendingArchive: false,
    hasFailedArchive: false,
    errorMessage: state.status === "error" ? state.message : null,
  };
}

function selectedSummary(items: LibraryItemSummary[], selectedItemId: string | null) {
  if (!selectedItemId) {
    return null;
  }
  return items.find((item) => item.id === selectedItemId) ?? items[0] ?? null;
}

function updatedSelectedDetail(
  detail: LibraryItemDetail | null,
  summary: LibraryItemSummary
): LibraryItemDetail | null {
  if (!detail || detail.summary.id !== summary.id) {
    return detail;
  }
  return {
    ...detail,
    summary,
  };
}

function hasArchiveStatus(
  items: LibraryItemSummary[],
  archiveStatus: LibraryItemSummary["archive_status"]
) {
  return items.some((item) => item.archive_status === archiveStatus);
}
