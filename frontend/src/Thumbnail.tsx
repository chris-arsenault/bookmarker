import type { LibraryItemSummary } from "./types";
import { itemKindLabel } from "./itemDisplay";

export function Thumbnail({
  item,
  thumbnailUrl,
}: {
  item: LibraryItemSummary;
  thumbnailUrl: string | null;
}) {
  if (thumbnailUrl) {
    return <img alt="" className="thumbnail" src={thumbnailUrl} />;
  }
  return (
    <div className="thumbnail fallback" aria-hidden="true">
      {item.thumbnail_s3_key ? "Snapshot" : itemKindLabel(item)}
    </div>
  );
}
