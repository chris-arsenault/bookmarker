import { useState, type KeyboardEvent } from "react";
import { ItemStatusControls } from "./ItemStatusControls";
import { SaveErrorModal } from "./SaveErrorModal";
import { TagEditor } from "./TagEditor";
import { itemFetchedTitle, itemSubtitle, itemTitle } from "./itemDisplay";
import type {
  LibraryItemDetail,
  LibraryItemSummary,
  TagCorpusEntry,
  UpdateItemRequest,
} from "./types";

type OrganizerDensity = "default" | "compact";
type SavingField = "title" | "notes" | "tags" | "watch" | "inbox" | null;

type OrganizerDraft = {
  title: string;
  notes: string;
  tags: string[];
};

export function ItemOrganizer({
  detail,
  availableTags,
  density,
  onUpdateItem,
}: {
  detail: LibraryItemDetail;
  availableTags: TagCorpusEntry[];
  density: OrganizerDensity;
  onUpdateItem: (itemId: string, request: UpdateItemRequest) => Promise<LibraryItemDetail>;
}) {
  const [draft, setDraft] = useState(() => draftFromDetail(detail));
  const [savingField, setSavingField] = useState<SavingField>(null);
  const [error, setError] = useState("");

  const savePatch = async (request: UpdateItemRequest, field: SavingField) => {
    setSavingField(field);
    setError("");
    try {
      const updated = await onUpdateItem(detail.summary.id, request);
      confirmReturnedPatch(updated, request);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Item update failed");
    } finally {
      setSavingField(null);
    }
  };

  return (
    <section className={density === "compact" ? "item-organizer compact" : "item-organizer"}>
      <div className="item-organizer-topline">
        <TitleEditor
          displayTitle={itemTitle(detail.summary)}
          disabled={savingField !== null}
          savedTitle={detail.summary.title}
          value={draft.title}
          onChange={(title) => setDraft({ ...draft, title })}
          onCommit={(title) => commitTitle(title, detail.summary.title, savePatch)}
        />
        <ItemStatusControls
          disabled={savingField !== null}
          summary={detail.summary}
          onInboxStatus={(inbox_status) => {
            void savePatch({ inbox_status }, "inbox");
          }}
          onWatchStatus={(watch_status) => {
            void savePatch({ watch_status }, "watch");
          }}
        />
      </div>
      <TitleMetadata summary={detail.summary} />
      <TagEditor
        availableTags={availableTags}
        disabled={savingField !== null}
        selectedTags={draft.tags}
        onChange={(tags) => {
          setDraft({ ...draft, tags });
          void savePatch({ tags }, "tags");
        }}
      />
      <NotesEditor
        disabled={savingField !== null}
        savedNotes={detail.notes}
        value={draft.notes}
        onChange={(notes) => setDraft({ ...draft, notes })}
        onCommit={(notes) => commitNotes(notes, detail.notes, savePatch)}
      />
      {error ? <SaveErrorModal message={error} onClose={() => setError("")} /> : null}
    </section>
  );
}

function TitleEditor({
  value,
  savedTitle,
  displayTitle,
  disabled,
  onChange,
  onCommit,
}: {
  value: string;
  savedTitle: string | null;
  displayTitle: string;
  disabled: boolean;
  onChange: (title: string) => void;
  onCommit: (title: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const visibleTitle = cleanTitle(value) ?? displayTitle;
  return (
    <h2 className="editable-title" id="detail-title">
      {editing ? (
        <input
          aria-label="Title"
          disabled={disabled}
          onBlur={() => {
            setEditing(false);
            onCommit(value);
          }}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={(event) => handleTitleKeyDown(event, savedTitle, onChange, setEditing)}
          placeholder={displayTitle}
          ref={(input) => {
            if (editing) {
              input?.focus();
            }
          }}
          type="text"
          value={value}
        />
      ) : (
        <button disabled={disabled} onClick={() => setEditing(true)} type="button">
          {visibleTitle}
        </button>
      )}
    </h2>
  );
}

function TitleMetadata({ summary }: { summary: LibraryItemSummary }) {
  const fetchedTitle = itemFetchedTitle(summary);
  return (
    <div className="item-title-metadata">
      <span>{itemSubtitle(summary)}</span>
      {fetchedTitle ? <span>Fetched: {fetchedTitle}</span> : null}
    </div>
  );
}

function NotesEditor({
  value,
  savedNotes,
  disabled,
  onChange,
  onCommit,
}: {
  value: string;
  savedNotes: string;
  disabled: boolean;
  onChange: (notes: string) => void;
  onCommit: (notes: string) => void;
}) {
  return (
    <textarea
      aria-label="Notes"
      className="notes-editor"
      disabled={disabled}
      onBlur={() => onCommit(value)}
      onChange={(event) => onChange(event.target.value)}
      placeholder="Notes"
      rows={notesRows(value, savedNotes)}
      value={value}
    />
  );
}

function handleTitleKeyDown(
  event: KeyboardEvent<HTMLInputElement>,
  savedTitle: string | null,
  onChange: (title: string) => void,
  setEditing: (editing: boolean) => void
) {
  if (event.key === "Enter") {
    event.currentTarget.blur();
  }
  if (event.key === "Escape") {
    onChange(savedTitle ?? "");
    setEditing(false);
  }
}

function commitTitle(
  title: string,
  savedTitle: string | null,
  savePatch: (request: UpdateItemRequest, field: SavingField) => Promise<void>
) {
  if (cleanTitle(title) !== savedTitle) {
    void savePatch({ title }, "title");
  }
}

function commitNotes(
  notes: string,
  savedNotes: string,
  savePatch: (request: UpdateItemRequest, field: SavingField) => Promise<void>
) {
  if (notes !== savedNotes) {
    void savePatch({ notes }, "notes");
  }
}

function confirmReturnedPatch(detail: LibraryItemDetail, request: UpdateItemRequest) {
  if ("title" in request && detail.summary.title !== cleanTitle(request.title ?? "")) {
    throw new Error("Title save did not persist. The API may need to be deployed.");
  }
  if ("notes" in request && detail.notes !== (request.notes ?? "")) {
    throw new Error("Notes save did not persist. The API may need to be deployed.");
  }
}

function draftFromDetail(detail: LibraryItemDetail): OrganizerDraft {
  return {
    title: detail.summary.title ?? "",
    notes: detail.notes,
    tags: detail.summary.tags.map((tag) => tag.display_name),
  };
}

function notesRows(value: string, savedNotes: string) {
  const lineCount = Math.max(value.split("\n").length, savedNotes.split("\n").length);
  return Math.min(Math.max(lineCount, 3), 6);
}

function cleanTitle(title: string) {
  const cleaned = title.trim();
  return cleaned ? cleaned : null;
}
