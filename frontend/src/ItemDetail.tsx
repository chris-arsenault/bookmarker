import { useCallback, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";
import { ImageItemDetail } from "./ImageDetail";
import { ItemOrganizer } from "./ItemOrganizer";
import { itemCopyLabel, itemFetchedTitle, itemSourceUrl } from "./itemDisplay";
import { useLibraryActions } from "./LibraryActionsContext";
import type { LibraryItemDetail, LibraryItemSummary, TagCorpusEntry } from "./types";

const markdownPlugins = [remarkGfm, remarkBreaks];

export function ItemDetail({
  detail,
  availableTags,
  onClose,
}: {
  detail: LibraryItemDetail | null;
  availableTags: TagCorpusEntry[];
  onClose: () => void;
}) {
  const { updateItem } = useLibraryActions();
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
        <ItemOrganizer
          availableTags={availableTags}
          detail={detail}
          density={isTextSnippet ? "compact" : "default"}
          key={summary.id}
          onUpdateItem={updateItem}
        />
        <DetailPrimary detail={detail} sourceUrl={sourceUrl} />
        <DetailActions detail={detail} onClose={onClose} sourceUrl={sourceUrl} />
      </aside>
    </div>
  );
}

function DetailPrimary({
  detail,
  sourceUrl,
}: {
  detail: LibraryItemDetail;
  sourceUrl: string | null;
}) {
  const { summary } = detail;
  if (summary.text) {
    return <TextSnippetDetail detail={detail} />;
  }
  if (summary.image) {
    return <ImageItemDetail detail={detail} />;
  }
  return <LinkDetailHeading sourceUrl={sourceUrl} summary={summary} />;
}

function TextSnippetDetail({ detail }: { detail: LibraryItemDetail }) {
  const { summary } = detail;
  if (!summary.text) {
    return null;
  }
  return (
    <section className="text-detail-summary" aria-label="Saved text">
      <div className="snippet-body snippet-body-primary markdown-snippet">
        <ReactMarkdown remarkPlugins={markdownPlugins}>{summary.text.plain_text}</ReactMarkdown>
      </div>
    </section>
  );
}

function LinkDetailHeading({
  summary,
  sourceUrl,
}: {
  summary: LibraryItemSummary;
  sourceUrl: string | null;
}) {
  const fetchedTitle = itemFetchedTitle(summary);
  return (
    <div className="link-detail-content">
      {fetchedTitle ? <p>Fetched title: {fetchedTitle}</p> : null}
      {sourceUrl ? (
        <a className="detail-source-url" href={sourceUrl} rel="noreferrer" target="_blank">
          {sourceUrl}
        </a>
      ) : null}
    </div>
  );
}

function DetailActions({
  detail,
  sourceUrl,
  onClose,
}: {
  detail: LibraryItemDetail;
  sourceUrl: string | null;
  onClose: () => void;
}) {
  const { copyItem, deleteItem: removeItem, openSource } = useLibraryActions();
  const { summary } = detail;
  return (
    <div className="detail-actions">
      {sourceUrl ? (
        <button className="primary-action" onClick={() => openSource(sourceUrl)} type="button">
          Open source
        </button>
      ) : null}
      <button className="secondary-action" onClick={() => copyItem(summary)} type="button">
        {itemCopyLabel(summary)}
      </button>
      <DeleteButton
        itemId={summary.id}
        key={summary.id}
        onDeleted={onClose}
        onDeleteItem={removeItem}
      />
    </div>
  );
}

function DeleteButton({
  itemId,
  onDeleted,
  onDeleteItem,
}: {
  itemId: string;
  onDeleted: () => void;
  onDeleteItem: (itemId: string) => Promise<void>;
}) {
  const [confirming, setConfirming] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState("");
  const cancelDelete = useCallback(() => setConfirming(false), []);
  const confirmDelete = useCallback(() => {
    deleteItem(itemId, onDeleteItem, onDeleted, setDeleting, setDeleteError).catch(() => {});
  }, [itemId, onDeleteItem, onDeleted]);
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
          onCancel={cancelDelete}
          onConfirm={confirmDelete}
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
    setDeleting(false);
    onDeleted();
  } catch (error) {
    setDeleteError(error instanceof Error ? error.message : "Delete failed");
    setDeleting(false);
  }
}
