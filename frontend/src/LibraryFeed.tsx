import type { KeyboardEvent } from "react";
import { formatDate } from "./dateDisplay";
import { StatusBadge } from "./StatusBadge";
import { Thumbnail } from "./Thumbnail";
import { itemCopyLabel, itemKindLabel, itemSubtitle, itemTitle } from "./itemDisplay";
import type { LibraryItemSummary } from "./types";

export function LibraryFeed({
  items,
  selectedItemId,
  thumbnailUrls,
  onSelectItem,
  onCopyItem,
}: {
  items: LibraryItemSummary[];
  selectedItemId: string | null;
  thumbnailUrls: Record<string, string>;
  onSelectItem: (itemId: string) => void;
  onCopyItem: (item: LibraryItemSummary) => void;
}) {
  if (items.length === 0) {
    return <p className="empty-state">No saved items</p>;
  }
  return (
    <div className="items-table-wrap">
      <table className="items-table" aria-label="Saved items">
        <thead>
          <tr>
            <th scope="col">Item</th>
            <th scope="col">Source</th>
            <th scope="col">Tags</th>
            <th scope="col">Added</th>
            <th scope="col">Status</th>
            <th scope="col">
              <span className="sr-only">Copy</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <ItemRow
              item={item}
              key={item.id}
              onCopyItem={onCopyItem}
              onSelectItem={onSelectItem}
              selected={item.id === selectedItemId}
              thumbnailUrl={thumbnailUrls[item.id] ?? null}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function ItemRow({
  item,
  selected,
  thumbnailUrl,
  onSelectItem,
  onCopyItem,
}: {
  item: LibraryItemSummary;
  selected: boolean;
  thumbnailUrl: string | null;
  onSelectItem: (itemId: string) => void;
  onCopyItem: (item: LibraryItemSummary) => void;
}) {
  return (
    <tr
      className={selected ? "item-row selected" : "item-row"}
      onClick={() => onSelectItem(item.id)}
      onKeyDown={(event) => openFromKeyboard(event, item.id, onSelectItem)}
      tabIndex={0}
    >
      <td className="item-main-cell">
        <Thumbnail item={item} thumbnailUrl={thumbnailUrl} />
        <div className="item-main-copy">
          <strong>{itemTitle(item)}</strong>
          <span>{itemKindLabel(item)}</span>
        </div>
      </td>
      <td>{itemSubtitle(item)}</td>
      <td>
        <span className="tag-list-inline">{tagSummary(item)}</span>
      </td>
      <td>{formatDate(item.created_at)}</td>
      <td>
        <StatusBadge status={item.archive_status} />
      </td>
      <td>
        <button
          aria-label={itemCopyLabel(item)}
          className="icon-button"
          onClick={(event) => {
            event.stopPropagation();
            onCopyItem(item);
          }}
          type="button"
        >
          <CopyIcon />
        </button>
      </td>
    </tr>
  );
}

function openFromKeyboard(
  event: KeyboardEvent<HTMLTableRowElement>,
  itemId: string,
  onSelectItem: (itemId: string) => void
) {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    onSelectItem(itemId);
  }
}

function tagSummary(item: LibraryItemSummary) {
  return item.tags.length > 0 ? item.tags.map((tag) => tag.display_name).join(", ") : "none";
}

function CopyIcon() {
  return (
    <svg aria-hidden="true" focusable="false" viewBox="0 0 24 24">
      <rect x="8" y="8" width="11" height="11" rx="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1" />
    </svg>
  );
}
