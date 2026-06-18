import { useState, type FormEvent } from "react";
import { selectedTagsFromForm, TagEditor } from "./TagEditor";
import type {
  InboxStatus,
  LibraryItemDetail,
  TagCorpusEntry,
  UpdateItemRequest,
  WatchStatus,
} from "./types";

type OrganizerDensity = "default" | "compact";

const watchOptions: { label: string; value: WatchStatus }[] = [
  { label: "Unwatched", value: "unwatched" },
  { label: "Watched", value: "watched" },
];

const inboxOptions: { label: string; value: InboxStatus }[] = [
  { label: "Unsorted", value: "unsorted" },
  { label: "Organized", value: "organized" },
];

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
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setSaving(true);
    setError("");
    try {
      await onUpdateItem(detail.summary.id, organizationRequest(new FormData(event.currentTarget)));
    } catch (err) {
      setError(err instanceof Error ? err.message : "organization update failed");
    } finally {
      setSaving(false);
    }
  };
  return (
    <form
      className={density === "compact" ? "item-organizer compact" : "item-organizer"}
      onSubmit={submit}
    >
      <h3>Edit item</h3>
      <label className="organizer-field">
        Notes
        <textarea
          defaultValue={detail.notes}
          disabled={saving}
          name="notes"
          rows={density === "compact" ? 2 : 4}
        />
      </label>
      <TagEditor
        availableTags={availableTags}
        disabled={saving}
        selectedTags={detail.summary.tags}
      />
      <StatusChoices
        current={detail.summary.watch_status}
        disabled={saving}
        name="watch_status"
        options={watchOptions}
        title="Watch status"
      />
      <StatusChoices
        current={detail.summary.inbox_status}
        disabled={saving}
        name="inbox_status"
        options={inboxOptions}
        title="Inbox status"
      />
      {error ? <p className="form-error">{error}</p> : null}
      <button className="primary-action" disabled={saving} type="submit">
        {saving ? "Saving" : "Save item"}
      </button>
    </form>
  );
}

function StatusChoices<T extends string>({
  title,
  name,
  options,
  current,
  disabled,
}: {
  title: string;
  name: string;
  options: { label: string; value: T }[];
  current: T;
  disabled: boolean;
}) {
  return (
    <fieldset className="status-options">
      <legend>{title}</legend>
      {options.map((option) => (
        <label key={option.value}>
          <input
            defaultChecked={option.value === current}
            disabled={disabled}
            name={name}
            type="radio"
            value={option.value}
          />
          <span>{option.label}</span>
        </label>
      ))}
    </fieldset>
  );
}

function organizationRequest(formData: FormData): UpdateItemRequest {
  return {
    watch_status: formValue(formData, "watch_status") as WatchStatus,
    inbox_status: formValue(formData, "inbox_status") as InboxStatus,
    notes: formValue(formData, "notes"),
    tags: selectedTagsFromForm(formData),
  };
}

function formValue(formData: FormData, name: string) {
  const value = formData.get(name);
  return typeof value === "string" ? value : "";
}
