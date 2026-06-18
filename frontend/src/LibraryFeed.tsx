import { StatusBadge } from "./StatusBadge";
import { Thumbnail } from "./Thumbnail";
import { itemCopyLabel, itemFeedMeta, itemTitle } from "./itemDisplay";
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
    <section className="feed-grid" aria-label="Saved items">
      {items.map((item) => (
        <article
          className={item.id === selectedItemId ? "feed-card selected" : "feed-card"}
          key={item.id}
        >
          <button className="feed-card-main" onClick={() => onSelectItem(item.id)} type="button">
            <Thumbnail item={item} thumbnailUrl={thumbnailUrls[item.id] ?? null} />
            <span className="feed-card-title">{itemTitle(item)}</span>
            <span className="feed-card-meta">
              {itemFeedMeta(item, formatDate(item.created_at))}
            </span>
            <StatusBadge status={item.archive_status} />
          </button>
          <button
            className="secondary-action feed-card-copy"
            onClick={() => onCopyItem(item)}
            type="button"
          >
            {itemCopyLabel(item)}
          </button>
        </article>
      ))}
    </section>
  );
}

export function formatDate(value: string) {
  return new Intl.DateTimeFormat("en-US", {
    month: "long",
    day: "numeric",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(value));
}
