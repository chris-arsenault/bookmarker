import type { LibraryItemSummary } from "./types";

export function itemTitle(item: LibraryItemSummary) {
  return item.title ?? item.text?.preview ?? item.url?.copy_url ?? "Untitled item";
}

export function itemSubtitle(item: LibraryItemSummary) {
  if (item.text) {
    return item.text.source_app ?? "Text snippet";
  }
  return item.author ?? item.platform ?? "Unknown source";
}

export function itemFeedMeta(item: LibraryItemSummary, formattedDate: string) {
  const source = item.text?.source_app ?? item.platform ?? itemKindLabel(item);
  return `${source} · ${formattedDate}`;
}

export function itemCopyLabel(item: LibraryItemSummary) {
  return item.text ? "Copy text" : "Copy link";
}

export function itemCopyValue(item: LibraryItemSummary) {
  return item.text?.plain_text ?? item.url?.copy_url ?? "";
}

export function itemSourceUrl(item: LibraryItemSummary) {
  return item.url?.copy_url ?? null;
}

export function itemKindLabel(item: LibraryItemSummary) {
  return item.item_kind === "text_snippet" ? "Snippet" : "Link";
}
