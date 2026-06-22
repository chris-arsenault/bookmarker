import { useEffect, useState } from "react";
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

function downloadName(summary: LibraryItemSummary) {
  return summary.image?.original_filename ?? `${summary.id}.image`;
}
