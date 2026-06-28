import { useCallback, useMemo, useState, type FormEvent } from "react";
import { desktopBridge } from "./desktopBridge";
import { QuickCaptureActions } from "./QuickCaptureActions";
import { SaveErrorModal } from "./SaveErrorModal";
import type {
  CaptureItemOutcome,
  CaptureLinkRequest,
  CaptureTextRequest,
  TagCorpusEntry,
} from "./types";

type SaveState = {
  kind: "idle" | "success" | "error";
  text: string;
};

type CaptureMode = "text" | "link";
type CaptureDraft = {
  mode: CaptureMode;
  text: string;
  title: string;
  url: string;
  tagInput: string;
  selectedTags: string[];
};

type CaptureCommands = {
  createLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
  createText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
};

type DraftUpdate = (patch: Partial<CaptureDraft>) => void;

const emptyDraft: CaptureDraft = {
  mode: "text",
  selectedTags: [],
  tagInput: "",
  text: "",
  title: "",
  url: "",
};

export function QuickTextCapture({
  tags,
  onCreateText,
  onCreateLink,
}: {
  tags: TagCorpusEntry[];
  onCreateText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  onCreateLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
}) {
  const {
    clearSavedDraft,
    toggleTag: toggleSelectedTag,
    update: updateDraft,
    value: draft,
  } = useCaptureDraft();
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<SaveState>({ kind: "idle", text: "" });
  const [errorDetail, setErrorDetail] = useState<string | null>(null);
  const popularTags = useMemo(() => tags.slice(0, 8), [tags]);
  const closeError = useCallback(() => setErrorDetail(null), []);
  const changeMode = useCallback((mode: CaptureMode) => updateDraft({ mode }), [updateDraft]);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await saveCapture({
      clearDraft: clearSavedDraft,
      commands: { createLink: onCreateLink, createText: onCreateText },
      draft,
      feedback: { setErrorDetail, setMessage, setSaving },
    });
  };

  return (
    <>
      <form className="quick-capture" onSubmit={submit}>
        <QuickCaptureField draft={draft} disabled={saving} onDraftChange={updateDraft} />
        <QuickCaptureActions
          canSave={canSaveDraft(draft)}
          message={message}
          mode={draft.mode}
          saving={saving}
          setMode={changeMode}
        />
        <QuickTagSelector
          draft={draft}
          disabled={saving}
          onDraftChange={updateDraft}
          onToggleTag={toggleSelectedTag}
          popularTags={popularTags}
        />
      </form>
      {errorDetail ? <SaveErrorModal message={errorDetail} onClose={closeError} /> : null}
    </>
  );
}

function QuickCaptureField({
  draft,
  disabled,
  onDraftChange,
}: {
  draft: CaptureDraft;
  disabled: boolean;
  onDraftChange: DraftUpdate;
}) {
  return (
    <label className="quick-field">
      <span>New item</span>
      <input
        aria-label="New item title"
        disabled={disabled}
        name="quick-title"
        onChange={(event) => onDraftChange({ title: event.currentTarget.value })}
        placeholder="Title"
        value={draft.title}
      />
      {draft.mode === "text" ? (
        <textarea
          aria-label="New text item"
          disabled={disabled}
          name="quick-text"
          onChange={(event) => onDraftChange({ text: event.currentTarget.value })}
          rows={5}
          value={draft.text}
        />
      ) : (
        <div className="quick-link-fields">
          <input
            aria-label="New link URL"
            disabled={disabled}
            name="quick-url"
            onChange={(event) => onDraftChange({ url: event.currentTarget.value })}
            placeholder="URL"
            type="url"
            value={draft.url}
          />
        </div>
      )}
    </label>
  );
}

function QuickTagSelector({
  popularTags,
  draft,
  disabled,
  onToggleTag,
  onDraftChange,
}: {
  popularTags: TagCorpusEntry[];
  draft: CaptureDraft;
  disabled: boolean;
  onToggleTag: (tag: string) => void;
  onDraftChange: DraftUpdate;
}) {
  return (
    <div aria-label="New item tags" className="quick-tags">
      {popularTags.length > 0 ? (
        <div className="quick-tag-chips">
          {popularTags.map((tag) => (
            <button
              aria-pressed={draft.selectedTags.includes(tag.display_name)}
              className={
                draft.selectedTags.includes(tag.display_name)
                  ? "quick-tag-chip selected"
                  : "quick-tag-chip"
              }
              disabled={disabled}
              key={tag.id}
              onClick={() => onToggleTag(tag.display_name)}
              type="button"
            >
              {tag.display_name}
            </button>
          ))}
        </div>
      ) : null}
      <label className="quick-tag-input">
        <span>Tags</span>
        <input
          disabled={disabled}
          name="quick-tags"
          onChange={(event) => onDraftChange({ tagInput: event.currentTarget.value })}
          value={draft.tagInput}
        />
      </label>
    </div>
  );
}

function useCaptureDraft() {
  const [value, setValue] = useState<CaptureDraft>(emptyDraft);
  const update = useCallback<DraftUpdate>((patch) => {
    setValue((current) => ({ ...current, ...patch }));
  }, []);
  const toggleSelectedTag = useCallback((tag: string) => {
    setValue((current) => ({ ...current, selectedTags: toggleTag(current.selectedTags, tag) }));
  }, []);
  const clearSavedDraft = useCallback((mode: CaptureMode) => {
    setValue((current) => ({
      ...current,
      selectedTags: [],
      tagInput: "",
      text: mode === "text" ? "" : current.text,
      title: "",
      url: mode === "link" ? "" : current.url,
    }));
  }, []);
  return {
    clearSavedDraft,
    toggleTag: toggleSelectedTag,
    update,
    value,
  };
}

async function saveCapture(options: {
  clearDraft: (mode: CaptureMode) => void;
  commands: CaptureCommands;
  draft: CaptureDraft;
  feedback: {
    setErrorDetail: (message: string | null) => void;
    setMessage: (message: SaveState) => void;
    setSaving: (saving: boolean) => void;
  };
}) {
  const { clearDraft, commands, draft, feedback } = options;
  if (!canSaveDraft(draft)) {
    feedback.setMessage({ kind: "error", text: emptyDraftMessage(draft.mode) });
    return;
  }
  feedback.setSaving(true);
  feedback.setMessage({ kind: "idle", text: "Saving" });
  feedback.setErrorDetail(null);
  try {
    const outcome = await createCapture(draft, commands);
    clearDraft(draft.mode);
    feedback.setMessage({ kind: "success", text: outcome.created ? "Saved" : "Already saved" });
  } catch (error) {
    const errorMessage = saveErrorMessage(error);
    feedback.setMessage({ kind: "error", text: "Save failed" });
    feedback.setErrorDetail(errorMessage);
  } finally {
    feedback.setSaving(false);
  }
}

async function createCapture(draft: CaptureDraft, commands: CaptureCommands) {
  const tags = selectedTagValues(draft);
  if (draft.mode === "text") {
    return commands.createText(await textRequest(draft.text, draft.title, tags));
  }
  return commands.createLink(linkRequest(draft.url.trim(), draft.title, tags));
}

function saveErrorMessage(error: unknown) {
  const message = error instanceof Error ? error.message.trim() : "";
  return message ? `Save failed: ${message}` : "Save failed: unknown error";
}

async function textRequest(
  plainText: string,
  title: string,
  tags: string[]
): Promise<CaptureTextRequest> {
  return {
    plain_text: plainText,
    title: optionalString(title),
    html: null,
    source_app: "Bookmarker",
    source_device: (await desktopBridge()?.platform()) ?? null,
    capture_method: "desktop_manual",
    tags,
    client_capture_id: randomId(),
  };
}

function linkRequest(url: string, title: string, tags: string[]): CaptureLinkRequest {
  return {
    url,
    title: optionalString(title),
    tags,
    client_capture_id: randomId(),
  };
}

function optionalString(value: string) {
  return value.trim() || null;
}

function canSaveDraft(draft: CaptureDraft) {
  return draft.mode === "text" ? draft.text.trim().length > 0 : draft.url.trim().length > 0;
}

function emptyDraftMessage(mode: CaptureMode) {
  return mode === "text" ? "Enter text" : "Enter link";
}

function selectedTagValues(draft: CaptureDraft) {
  return dedupeTags([...draft.selectedTags, ...draft.tagInput.split(",")]);
}

function dedupeTags(tags: string[]) {
  const selected = new Map<string, string>();
  tags.map((tag) => tag.trim()).forEach((tag) => addTagValue(selected, tag));
  return [...selected.values()];
}

function addTagValue(selected: Map<string, string>, tag: string) {
  const key = tag.toLowerCase();
  if (tag.length > 0 && !selected.has(key)) {
    selected.set(key, tag);
  }
}

function toggleTag(selectedTags: string[], tag: string) {
  return selectedTags.includes(tag)
    ? selectedTags.filter((selected) => selected !== tag)
    : [...selectedTags, tag];
}

function randomId() {
  return globalThis.crypto?.randomUUID?.() ?? fallbackId();
}

let fallbackCounter = 0;

function fallbackId() {
  fallbackCounter += 1;
  return `capture-${Date.now()}-${fallbackCounter}`;
}
