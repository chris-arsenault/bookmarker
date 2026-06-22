import type { LibraryState } from "./libraryState";
import type { ApiDateTime, LibraryItemDetail, LibraryItemSummary, LibraryUpdates } from "./types";
import { parseApiDate } from "./dateDisplay";

export const ACTIVE_UPDATE_POLL_MS = 8_000;
export const IDLE_UPDATE_POLL_MS = 30_000;

export function updatePollInterval(items: LibraryItemSummary[]) {
  return items.some((item) => item.archive_status === "pending")
    ? ACTIVE_UPDATE_POLL_MS
    : IDLE_UPDATE_POLL_MS;
}

export function applyLibraryUpdates(state: LibraryState, updates: LibraryUpdates): LibraryState {
  if (state.status !== "ready") {
    return state;
  }
  const items = mergeUpdatedItems(
    withoutDeletedItems(state.items, updates.deleted_item_ids),
    updates.items
  );
  return {
    ...state,
    items,
    tags: updates.tags,
    selectedItemId: updatedSelectedItemId(state.selectedItemId, items, updates.deleted_item_ids),
    selectedDetail: mergeSelectedDetail(state.selectedDetail, updates),
  };
}

export function updateCursorString(value: ApiDateTime) {
  return parseApiDate(value)?.toISOString() ?? new Date().toISOString();
}

function mergeUpdatedItems(currentItems: LibraryItemSummary[], updatedItems: LibraryItemSummary[]) {
  const byId = new Map(currentItems.map((item) => [item.id, item]));
  for (const item of updatedItems) {
    byId.set(item.id, item);
  }
  return [...byId.values()].sort(compareCreatedDesc);
}

function withoutDeletedItems(items: LibraryItemSummary[], deletedItemIds: string[]) {
  const deleted = new Set(deletedItemIds);
  return items.filter((item) => !deleted.has(item.id));
}

function updatedSelectedItemId(
  selectedItemId: string | null,
  items: LibraryItemSummary[],
  deletedItemIds: string[]
) {
  if (!selectedItemId || deletedItemIds.includes(selectedItemId)) {
    return items[0]?.id ?? null;
  }
  return items.some((item) => item.id === selectedItemId) ? selectedItemId : (items[0]?.id ?? null);
}

function mergeSelectedDetail(
  detail: LibraryItemDetail | null,
  updates: LibraryUpdates
): LibraryItemDetail | null {
  if (!detail || updates.deleted_item_ids.includes(detail.summary.id)) {
    return null;
  }
  const summary = detail ? updates.items.find((item) => item.id === detail.summary.id) : undefined;
  return summary ? { summary, notes: detail.notes } : detail;
}

function compareCreatedDesc(left: LibraryItemSummary, right: LibraryItemSummary) {
  return dateValue(right.created_at) - dateValue(left.created_at);
}

function dateValue(value: LibraryItemSummary["created_at"]) {
  return parseApiDate(value)?.getTime() ?? 0;
}
