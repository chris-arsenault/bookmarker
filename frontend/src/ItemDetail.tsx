import { useState } from "react";
import { formatDate } from "./LibraryFeed";
import { ItemOrganizer } from "./ItemOrganizer";
import { StatusBadge } from "./StatusBadge";
import { Thumbnail } from "./Thumbnail";
import { itemCopyLabel, itemSourceUrl, itemSubtitle, itemTitle } from "./itemDisplay";
import type { ItemTag, LibraryItemDetail, TagCorpusEntry, UpdateItemRequest } from "./types";

export function ItemDetail({
  detail,
  availableTags,
  onCopyLink,
  onOpenSource,
  onUpdateItem,
  onDeleteItem,
}: {
  detail: LibraryItemDetail | null;
  availableTags: TagCorpusEntry[];
  onCopyLink: (item: LibraryItemDetail["summary"]) => void;
  onOpenSource: (url: string) => void;
  onUpdateItem: (itemId: string, request: UpdateItemRequest) => Promise<LibraryItemDetail>;
  onDeleteItem?: (itemId: string) => Promise<void>;
}) {
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState("");
  if (!detail) {
    return <aside className="detail-panel empty-state">Select a saved item</aside>;
  }
  const { summary } = detail;
  const sourceUrl = itemSourceUrl(summary);
  return (
    <aside className="detail-panel">
      <Thumbnail item={summary} thumbnailUrl={null} />
      <div className="detail-heading">
        <StatusBadge status={summary.archive_status} />
        <h2>{itemTitle(summary)}</h2>
        <p>{itemSubtitle(summary)}</p>
      </div>
      {summary.text ? <pre className="snippet-body">{summary.text.plain_text}</pre> : null}
      <DetailMeta detail={detail} />
      <DetailTags tags={summary.tags} />
      <p className="notes">{detail.notes}</p>
      <ItemOrganizer
        availableTags={availableTags}
        detail={detail}
        key={summary.id}
        onUpdateItem={onUpdateItem}
      />
      <DetailActions
        deleting={deleting}
        detail={detail}
        onCopyLink={onCopyLink}
        onDeleteItem={onDeleteItem}
        onOpenSource={onOpenSource}
        setDeleteError={setDeleteError}
        setDeleting={setDeleting}
        sourceUrl={sourceUrl}
      />
      {deleteError ? <p className="form-error">{deleteError}</p> : null}
    </aside>
  );
}

function DetailMeta({ detail }: { detail: LibraryItemDetail }) {
  const { summary } = detail;
  return (
    <dl className="detail-list">
      <div>
        <dt>Added</dt>
        <dd>{formatDate(summary.created_at)}</dd>
      </div>
      <div>
        <dt>Watch status</dt>
        <dd>{summary.watch_status}</dd>
      </div>
      <div>
        <dt>Inbox status</dt>
        <dd>{summary.inbox_status}</dd>
      </div>
    </dl>
  );
}

function DetailTags({ tags }: { tags: ItemTag[] }) {
  return (
    <div className="tag-row">
      {tags.map((tag) => (
        <span className="tag-chip" key={tag.id}>
          {tag.display_name}
        </span>
      ))}
    </div>
  );
}

function DetailActions({
  detail,
  sourceUrl,
  deleting,
  onCopyLink,
  onOpenSource,
  onDeleteItem,
  setDeleting,
  setDeleteError,
}: {
  detail: LibraryItemDetail;
  sourceUrl: string | null;
  deleting: boolean;
  onCopyLink: (item: LibraryItemDetail["summary"]) => void;
  onOpenSource: (url: string) => void;
  onDeleteItem?: (itemId: string) => Promise<void>;
  setDeleting: (deleting: boolean) => void;
  setDeleteError: (message: string) => void;
}) {
  const { summary } = detail;
  return (
    <div className="detail-actions">
      {sourceUrl ? (
        <button className="primary-action" onClick={() => onOpenSource(sourceUrl)} type="button">
          Open source
        </button>
      ) : null}
      <button className="secondary-action" onClick={() => onCopyLink(summary)} type="button">
        {itemCopyLabel(summary)}
      </button>
      <DeleteButton
        deleting={deleting}
        itemId={summary.id}
        onDeleteItem={onDeleteItem}
        setDeleteError={setDeleteError}
        setDeleting={setDeleting}
      />
    </div>
  );
}

function DeleteButton({
  itemId,
  deleting,
  onDeleteItem,
  setDeleting,
  setDeleteError,
}: {
  itemId: string;
  deleting: boolean;
  onDeleteItem?: (itemId: string) => Promise<void>;
  setDeleting: (deleting: boolean) => void;
  setDeleteError: (message: string) => void;
}) {
  if (!onDeleteItem) {
    return null;
  }
  return (
    <button
      className="danger-action"
      disabled={deleting}
      onClick={() => deleteItem(itemId, onDeleteItem, setDeleting, setDeleteError).catch(() => {})}
      type="button"
    >
      {deleting ? "Deleting" : "Delete item"}
    </button>
  );
}

async function deleteItem(
  itemId: string,
  onDeleteItem: (itemId: string) => Promise<void>,
  setDeleting: (deleting: boolean) => void,
  setDeleteError: (message: string) => void
) {
  if (!window.confirm("Delete this item?")) {
    return;
  }
  setDeleting(true);
  setDeleteError("");
  try {
    await onDeleteItem(itemId);
  } catch (error) {
    setDeleteError(error instanceof Error ? error.message : "Delete failed");
  } finally {
    setDeleting(false);
  }
}
