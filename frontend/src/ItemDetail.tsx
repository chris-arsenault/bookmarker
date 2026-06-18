import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";
import { formatDate } from "./dateDisplay";
import { ItemOrganizer } from "./ItemOrganizer";
import { StatusBadge } from "./StatusBadge";
import { Thumbnail } from "./Thumbnail";
import { itemCopyLabel, itemSourceUrl, itemSubtitle, itemTitle } from "./itemDisplay";
import type {
  ItemTag,
  LibraryItemDetail,
  LibraryItemSummary,
  TagCorpusEntry,
  UpdateItemRequest,
} from "./types";

const markdownPlugins = [remarkGfm, remarkBreaks];

export function ItemDetail({
  detail,
  availableTags,
  onClose,
  onCopyLink,
  onOpenSource,
  onUpdateItem,
  onDeleteItem,
}: {
  detail: LibraryItemDetail | null;
  availableTags: TagCorpusEntry[];
  onClose: () => void;
  onCopyLink: (item: LibraryItemDetail["summary"]) => void;
  onOpenSource: (url: string) => void;
  onUpdateItem: (itemId: string, request: UpdateItemRequest) => Promise<LibraryItemDetail>;
  onDeleteItem?: (itemId: string) => Promise<void>;
}) {
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState("");
  if (!detail) {
    return null;
  }
  const { summary } = detail;
  const sourceUrl = itemSourceUrl(summary);
  const isTextSnippet = Boolean(summary.text);
  return (
    <div className="modal-backdrop">
      <aside
        aria-labelledby="detail-title"
        aria-modal="true"
        className={isTextSnippet ? "detail-modal text-detail-modal" : "detail-modal"}
        role="dialog"
      >
        <button aria-label="Close detail" className="modal-close" onClick={onClose} type="button">
          &times;
        </button>
        {isTextSnippet ? (
          <TextSnippetDetail detail={detail} />
        ) : (
          <LinkDetailHeading summary={summary} />
        )}
        {isTextSnippet ? null : <DetailMeta detail={detail} />}
        <DetailTags tags={summary.tags} />
        {!isTextSnippet && detail.notes ? <p className="notes">{detail.notes}</p> : null}
        <ItemOrganizer
          availableTags={availableTags}
          detail={detail}
          density={isTextSnippet ? "compact" : "default"}
          key={summary.id}
          onUpdateItem={onUpdateItem}
        />
        <DetailActions
          deleteError={deleteError}
          deleting={deleting}
          detail={detail}
          onClose={onClose}
          onCopyLink={onCopyLink}
          onDeleteItem={onDeleteItem}
          onOpenSource={onOpenSource}
          setDeleteError={setDeleteError}
          setDeleting={setDeleting}
          sourceUrl={sourceUrl}
        />
        {deleteError ? <p className="form-error">{deleteError}</p> : null}
      </aside>
    </div>
  );
}

function TextSnippetDetail({ detail }: { detail: LibraryItemDetail }) {
  const { summary } = detail;
  if (!summary.text) {
    return null;
  }
  return (
    <section className="text-detail-summary" aria-label="Saved text">
      <div className="text-detail-heading">
        <p className="eyebrow">Text</p>
        <h2 id="detail-title">Saved text</h2>
        <p>{textSnippetMeta(summary)}</p>
      </div>
      <div className="snippet-body snippet-body-primary markdown-snippet">
        <ReactMarkdown remarkPlugins={markdownPlugins}>{summary.text.plain_text}</ReactMarkdown>
      </div>
    </section>
  );
}

function LinkDetailHeading({ summary }: { summary: LibraryItemSummary }) {
  return (
    <>
      <Thumbnail item={summary} thumbnailUrl={null} />
      <div className="detail-heading">
        <StatusBadge status={summary.archive_status} />
        <h2 id="detail-title">{itemTitle(summary)}</h2>
        <p>{itemSubtitle(summary)}</p>
      </div>
    </>
  );
}

function textSnippetMeta(summary: LibraryItemSummary) {
  const source = summary.text?.source_app ?? summary.text?.source_device ?? "Manual capture";
  return `${source} · ${formatDate(summary.created_at)}`;
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
  if (tags.length === 0) {
    return null;
  }
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
  deleteError,
  deleting,
  onClose,
  onCopyLink,
  onOpenSource,
  onDeleteItem,
  setDeleting,
  setDeleteError,
}: {
  detail: LibraryItemDetail;
  sourceUrl: string | null;
  deleteError: string;
  deleting: boolean;
  onClose: () => void;
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
        deleteError={deleteError}
        deleting={deleting}
        itemId={summary.id}
        onDeleted={onClose}
        onDeleteItem={onDeleteItem}
        setDeleteError={setDeleteError}
        setDeleting={setDeleting}
      />
    </div>
  );
}

function DeleteButton({
  itemId,
  deleteError,
  deleting,
  onDeleted,
  onDeleteItem,
  setDeleting,
  setDeleteError,
}: {
  itemId: string;
  deleteError: string;
  deleting: boolean;
  onDeleted: () => void;
  onDeleteItem?: (itemId: string) => Promise<void>;
  setDeleting: (deleting: boolean) => void;
  setDeleteError: (message: string) => void;
}) {
  const [confirming, setConfirming] = useState(false);
  if (!onDeleteItem) {
    return null;
  }
  return (
    <>
      <button
        className="danger-action"
        disabled={deleting}
        onClick={() => {
          setDeleteError("");
          setConfirming(true);
        }}
        type="button"
      >
        {deleting ? "Deleting" : "Delete item"}
      </button>
      {confirming ? (
        <DeleteConfirmModal
          deleting={deleting}
          error={deleteError}
          onCancel={() => setConfirming(false)}
          onConfirm={() => {
            deleteItem(itemId, onDeleteItem, onDeleted, setDeleting, setDeleteError).catch(
              () => {}
            );
          }}
        />
      ) : null}
    </>
  );
}

function DeleteConfirmModal({
  deleting,
  error,
  onCancel,
  onConfirm,
}: {
  deleting: boolean;
  error: string;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="modal-backdrop confirm-backdrop">
      <section
        aria-labelledby="delete-confirm-title"
        aria-modal="true"
        className="confirm-modal"
        role="dialog"
      >
        <h2 id="delete-confirm-title">Delete this item?</h2>
        <p>This removes it from the vault.</p>
        {error ? <p className="form-error">{error}</p> : null}
        <div className="confirm-actions">
          <button className="secondary-action" disabled={deleting} onClick={onCancel} type="button">
            Cancel
          </button>
          <button className="danger-action" disabled={deleting} onClick={onConfirm} type="button">
            {deleting ? "Deleting" : "Delete permanently"}
          </button>
        </div>
      </section>
    </div>
  );
}

async function deleteItem(
  itemId: string,
  onDeleteItem: (itemId: string) => Promise<void>,
  onDeleted: () => void,
  setDeleting: (deleting: boolean) => void,
  setDeleteError: (message: string) => void
): Promise<void> {
  setDeleting(true);
  setDeleteError("");
  try {
    await onDeleteItem(itemId);
    onDeleted();
  } catch (error) {
    setDeleteError(error instanceof Error ? error.message : "Delete failed");
    setDeleting(false);
  }
}
