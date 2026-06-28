import { useEffect, useRef, useState } from "react";
import type { ImageAccessTarget, ImageUploadStatus, LibraryItemDetail } from "./types";
import { useImageAccessLoader, type ImageAccessLoader } from "./ImageAccessContext";

type ImageAccessState = {
  itemId: string;
  status: "idle" | "loading" | "ready" | "error";
  access: ImageAccessTarget | null;
};

export function ImageItemDetail({ detail }: { detail: LibraryItemDetail }) {
  const loadImageAccess = useImageAccessLoader();
  const { summary } = detail;
  const image = summary.image;
  const imageAccess = useImageAccessTarget(
    summary.id,
    image?.upload_status === "uploaded" ? loadImageAccess : null
  );
  if (!image) {
    return null;
  }
  return (
    <section className="image-detail-summary" aria-label="Saved image">
      <ImagePreview
        imageUrl={imageAccess.access?.view_url ?? null}
        status={imageAccess.status}
        uploadStatus={image.upload_status}
      />
      {imageAccess.access ? (
        <a
          className="secondary-action image-download"
          download={imageAccess.access.download_name}
          href={imageAccess.access.download_url}
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
  uploadStatus,
}: {
  imageUrl: string | null;
  status: ImageAccessState["status"];
  uploadStatus: ImageUploadStatus;
}) {
  if (imageUrl) {
    return <img alt="" className="image-detail-preview" src={imageUrl} />;
  }
  if (uploadStatus === "pending") {
    return <div className="image-detail-placeholder">Image upload pending</div>;
  }
  if (uploadStatus === "failed") {
    return <div className="image-detail-placeholder">Image upload failed</div>;
  }
  return (
    <div className="image-detail-placeholder">
      {status === "error" ? "Image unavailable" : "Loading image"}
    </div>
  );
}

function useImageAccessTarget(itemId: string, loadImageAccess: ImageAccessLoader | null) {
  const loadImageRef = useRef(loadImageAccess);
  const canLoadImage = Boolean(loadImageAccess);
  const [state, setState] = useState<ImageAccessState>({
    itemId,
    status: canLoadImage ? "loading" : "idle",
    access: null,
  });
  useEffect(() => {
    loadImageRef.current = loadImageAccess;
  }, [loadImageAccess]);
  useEffect(() => {
    const loadImage = loadImageRef.current;
    if (!canLoadImage || !loadImage) {
      return;
    }
    let active = true;
    setState({ itemId, status: "loading", access: null });
    loadImage(itemId)
      .then((access) => {
        if (active) {
          setState({ itemId, status: "ready", access });
        }
      })
      .catch(() => {
        if (active) {
          setState({ itemId, status: "error", access: null });
        }
      });
    return () => {
      active = false;
    };
  }, [canLoadImage, itemId]);
  if (!canLoadImage) {
    return { status: "idle" as const, access: null };
  }
  return state.itemId === itemId ? state : { status: "loading" as const, access: null };
}
