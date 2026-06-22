import { useMemo, useState, type FormEvent } from "react";
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

export function QuickTextCapture({
  tags,
  onCreateText,
  onCreateLink,
}: {
  tags: TagCorpusEntry[];
  onCreateText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  onCreateLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
}) {
  const [mode, setMode] = useState<CaptureMode>("text");
  const [text, setText] = useState("");
  const [url, setUrl] = useState("");
  const [title, setTitle] = useState("");
  const [tagInput, setTagInput] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<SaveState>({ kind: "idle", text: "" });
  const [errorDetail, setErrorDetail] = useState<string | null>(null);
  const popularTags = useMemo(() => tags.slice(0, 8), [tags]);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await saveCapture({
      mode,
      text,
      url,
      title,
      tagInput,
      selectedTags,
      onCreateText,
      onCreateLink,
      setText,
      setUrl,
      setTitle,
      setTagInput,
      setSelectedTags,
      setSaving,
      setMessage,
      setErrorDetail,
    });
  };

  return (
    <>
      <form className="quick-capture" onSubmit={submit}>
        <QuickCaptureField
          disabled={saving}
          mode={mode}
          setText={setText}
          setUrl={setUrl}
          setTitle={setTitle}
          text={text}
          title={title}
          url={url}
        />
        <QuickCaptureActions
          canSave={canSaveDraft(mode, text, url)}
          message={message}
          mode={mode}
          saving={saving}
          setMode={setMode}
        />
        <QuickTagSelector
          disabled={saving}
          onTagInputChange={setTagInput}
          onToggleTag={(tag) => setSelectedTags(toggleTag(selectedTags, tag))}
          popularTags={popularTags}
          selectedTags={selectedTags}
          tagInput={tagInput}
        />
      </form>
      {errorDetail ? (
        <SaveErrorModal message={errorDetail} onClose={() => setErrorDetail(null)} />
      ) : null}
    </>
  );
}

function QuickCaptureField({
  mode,
  text,
  url,
  title,
  disabled,
  setText,
  setUrl,
  setTitle,
}: {
  mode: CaptureMode;
  text: string;
  url: string;
  title: string;
  disabled: boolean;
  setText: (value: string) => void;
  setUrl: (value: string) => void;
  setTitle: (value: string) => void;
}) {
  return (
    <label className="quick-field">
      <span>New item</span>
      <input
        aria-label="New item title"
        disabled={disabled}
        name="quick-title"
        onChange={(event) => setTitle(event.currentTarget.value)}
        placeholder="Title"
        value={title}
      />
      {mode === "text" ? (
        <textarea
          aria-label="New text item"
          disabled={disabled}
          name="quick-text"
          onChange={(event) => setText(event.currentTarget.value)}
          rows={5}
          value={text}
        />
      ) : (
        <div className="quick-link-fields">
          <input
            aria-label="New link URL"
            disabled={disabled}
            name="quick-url"
            onChange={(event) => setUrl(event.currentTarget.value)}
            placeholder="URL"
            type="url"
            value={url}
          />
        </div>
      )}
    </label>
  );
}

function QuickTagSelector({
  popularTags,
  selectedTags,
  tagInput,
  disabled,
  onToggleTag,
  onTagInputChange,
}: {
  popularTags: TagCorpusEntry[];
  selectedTags: string[];
  tagInput: string;
  disabled: boolean;
  onToggleTag: (tag: string) => void;
  onTagInputChange: (value: string) => void;
}) {
  return (
    <div aria-label="New item tags" className="quick-tags">
      {popularTags.length > 0 ? (
        <div className="quick-tag-chips">
          {popularTags.map((tag) => (
            <button
              aria-pressed={selectedTags.includes(tag.display_name)}
              className={
                selectedTags.includes(tag.display_name)
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
          onChange={(event) => onTagInputChange(event.currentTarget.value)}
          value={tagInput}
        />
      </label>
    </div>
  );
}

async function saveCapture(options: {
  mode: CaptureMode;
  text: string;
  url: string;
  title: string;
  tagInput: string;
  selectedTags: string[];
  onCreateText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  onCreateLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
  setText: (value: string) => void;
  setUrl: (value: string) => void;
  setTitle: (value: string) => void;
  setTagInput: (value: string) => void;
  setSelectedTags: (tags: string[]) => void;
  setSaving: (saving: boolean) => void;
  setMessage: (message: SaveState) => void;
  setErrorDetail: (message: string | null) => void;
}) {
  if (!canSaveDraft(options.mode, options.text, options.url)) {
    options.setMessage({ kind: "error", text: emptyDraftMessage(options.mode) });
    return;
  }
  options.setSaving(true);
  options.setMessage({ kind: "idle", text: "Saving" });
  options.setErrorDetail(null);
  try {
    const outcome = await createCapture(options);
    clearDraft(options);
    options.setMessage({ kind: "success", text: outcome.created ? "Saved" : "Already saved" });
  } catch (error) {
    const errorMessage = saveErrorMessage(error);
    options.setMessage({ kind: "error", text: "Save failed" });
    options.setErrorDetail(errorMessage);
  } finally {
    options.setSaving(false);
  }
}

async function createCapture(options: {
  mode: CaptureMode;
  text: string;
  url: string;
  title: string;
  tagInput: string;
  selectedTags: string[];
  onCreateText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  onCreateLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
}) {
  const tags = selectedTagValues(options.selectedTags, options.tagInput);
  if (options.mode === "text") {
    return options.onCreateText(await textRequest(options.text, options.title, tags));
  }
  return options.onCreateLink(linkRequest(options.url.trim(), options.title, tags));
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

function canSaveDraft(mode: CaptureMode, text: string, url: string) {
  return mode === "text" ? text.trim().length > 0 : url.trim().length > 0;
}

function emptyDraftMessage(mode: CaptureMode) {
  return mode === "text" ? "Enter text" : "Enter link";
}

function selectedTagValues(selectedTags: string[], tagInput: string) {
  return dedupeTags([...selectedTags, ...tagInput.split(",")]);
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

function clearDraft(options: {
  mode: CaptureMode;
  setText: (value: string) => void;
  setUrl: (value: string) => void;
  setTitle: (value: string) => void;
  setTagInput: (value: string) => void;
  setSelectedTags: (tags: string[]) => void;
}) {
  if (options.mode === "text") {
    options.setText("");
  } else {
    options.setUrl("");
  }
  options.setTitle("");
  options.setTagInput("");
  options.setSelectedTags([]);
}

function randomId() {
  return globalThis.crypto?.randomUUID?.() ?? fallbackId();
}

let fallbackCounter = 0;

function fallbackId() {
  fallbackCounter += 1;
  return `capture-${Date.now()}-${fallbackCounter}`;
}
