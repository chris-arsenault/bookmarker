import { useEffect, useState } from "react";
import { formatDate } from "./dateDisplay";
import { itemTitle } from "./itemDisplay";
import { StatusBadge } from "./StatusBadge";
import type { LibraryItemDetail, LibraryItemSummary } from "./types";

type ImageObjectState = {
  itemId: string;
  status: "idle" | "loading" | "ready" | "error";
  url: string | null;
};

export function ImageItemDetail({
  detail,
  onLoadImage,
}: {
  detail: LibraryItemDetail;
  onLoadImage?: (itemId: string) => Promise<Blob>;
}) {
  const { summary } = detail;
  const image = summary.image;
  const imageUrl = useImageObjectUrl(summary.id, onLoadImage);
  if (!image) {
    return null;
  }
  return (
    <section className="image-detail-summary" aria-label="Saved image">
      <div className="detail-heading">
        <StatusBadge status={summary.archive_status} />
        <h2 id="detail-title">{itemTitle(summary)}</h2>
        <p>{imageMeta(summary)}</p>
      </div>
      <ImagePreview imageUrl={imageUrl.url} status={imageUrl.status} />
      {imageUrl.url ? (
        <a
          className="secondary-action image-download"
          download={downloadName(summary)}
          href={imageUrl.url}
        >
          Download image
        </a>
      ) : null}
    </section>
  );
}

function ImagePreview({
  imageUrl,
  status,
}: {
  imageUrl: string | null;
  status: ImageObjectState["status"];
}) {
  if (imageUrl) {
    return <img alt="" className="image-detail-preview" src={imageUrl} />;
  }
  return (
    <div className="image-detail-placeholder">
      {status === "error" ? "Image unavailable" : "Loading image"}
    </div>
  );
}

function useImageObjectUrl(
  itemId: string,
  onLoadImage: ((itemId: string) => Promise<Blob>) | undefined
) {
  const [state, setState] = useState<ImageObjectState>({
    itemId,
    status: onLoadImage ? "loading" : "idle",
    url: null,
  });
  useEffect(() => {
    if (!onLoadImage) {
      return;
    }
    let active = true;
    let objectUrl: string | null = null;
    onLoadImage(itemId)
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob);
        if (active) {
          setState({ itemId, status: "ready", url: objectUrl });
        } else {
          URL.revokeObjectURL(objectUrl);
        }
      })
      .catch(() => {
        if (active) {
          setState({ itemId, status: "error", url: null });
        }
      });
    return () => {
      active = false;
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, [itemId, onLoadImage]);
  if (!onLoadImage) {
    return { status: "idle" as const, url: null };
  }
  return state.itemId === itemId ? state : { status: "loading" as const, url: null };
}

function imageMeta(summary: LibraryItemSummary) {
  const image = summary.image;
  const source = image?.source_app ?? image?.source_device ?? "Image";
  const size = image?.byte_size ? ` · ${formatByteSize(image.byte_size)}` : "";
  return `${source} · ${formatDate(summary.created_at)}${size}`;
}

function formatByteSize(bytes: number) {
  if (bytes >= 1_000_000) {
    return `${(bytes / 1_000_000).toFixed(1)} MB`;
  }
  if (bytes >= 1_000) {
    return `${Math.round(bytes / 1_000)} KB`;
  }
  return `${bytes} B`;
}

function downloadName(summary: LibraryItemSummary) {
  return summary.image?.original_filename ?? `${summary.id}.image`;
}
