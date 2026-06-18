import type { LibraryItemSummary } from "./types";
import { itemCopyValue } from "./itemDisplay";

export type ClipboardWriter = {
  writeText: (value: string) => Promise<void> | void;
};

export async function copyCanonicalLink(
  item: LibraryItemSummary,
  clipboard: ClipboardWriter = browserClipboard()
) {
  const value = itemCopyValue(item);
  await clipboard.writeText(value);
  return value;
}

function browserClipboard(): ClipboardWriter {
  return navigator.clipboard;
}
