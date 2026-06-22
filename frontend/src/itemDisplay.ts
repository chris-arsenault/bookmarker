import type { LibraryItemSummary } from "./types";

export function itemTitle(item: LibraryItemSummary) {
  return (
    item.title ??
    item.fetched_title ??
    item.text?.preview ??
    item.image?.original_filename ??
    item.url?.copy_url ??
    "Untitled item"
  );
}

export function itemFetchedTitle(item: LibraryItemSummary) {
  return item.title && item.fetched_title && item.fetched_title !== item.title
    ? item.fetched_title
    : null;
}

export function itemSubtitle(item: LibraryItemSummary) {
  if (item.text) {
    return item.text.source_app ?? "Text snippet";
  }
  if (item.image) {
    return item.image.source_app ?? "Image";
  }
  return item.author ?? item.platform ?? "Unknown source";
}

export function itemFeedMeta(item: LibraryItemSummary, formattedDate: string) {
  const source =
    item.text?.source_app ?? item.image?.source_app ?? item.platform ?? itemKindLabel(item);
  return `${source} · ${formattedDate}`;
}

export function itemCopyLabel(item: LibraryItemSummary) {
  if (item.text) {
    return "Copy text";
  }
  return item.image ? "Copy image name" : "Copy link";
}

export function itemCopyValue(item: LibraryItemSummary) {
  return (
    item.text?.plain_text ?? item.url?.copy_url ?? item.image?.original_filename ?? itemTitle(item)
  );
}

export function itemSourceUrl(item: LibraryItemSummary) {
  return item.url?.copy_url ?? null;
}

export function itemKindLabel(item: LibraryItemSummary) {
  if (item.item_kind === "text_snippet") {
    return "Snippet";
  }
  return item.item_kind === "image" ? "Image" : "Link";
}
