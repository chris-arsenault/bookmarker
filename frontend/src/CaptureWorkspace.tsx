import { useMemo, useState, type FormEvent } from "react";
import { desktopBridge } from "./desktopBridge";
import type {
  CaptureItemOutcome,
  CaptureLinkRequest,
  CaptureTextRequest,
  TagCorpusEntry,
} from "./types";

type CaptureMode = "text" | "link";

type CaptureWorkspaceProps = {
  tags: TagCorpusEntry[];
  onCreateText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  onCreateLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
};

type SaveMessage = {
  kind: "idle" | "success" | "error";
  text: string;
};

export function CaptureWorkspace({ tags, onCreateText, onCreateLink }: CaptureWorkspaceProps) {
  const [mode, setMode] = useState<CaptureMode>("text");
  const [text, setText] = useState("");
  const [url, setUrl] = useState("");
  const [tagInput, setTagInput] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<SaveMessage>({ kind: "idle", text: "Ready" });
  const popularTags = useMemo(() => tags.slice(0, 8), [tags]);
  const canSave = canSaveDraft(mode, text, url);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await saveCapture({
      mode,
      text,
      url,
      tagInput,
      selectedTags,
      onCreateText,
      onCreateLink,
      setSaving,
      setMessage,
      clearDraft: () => clearDraft(mode, setText, setUrl, setTagInput, setSelectedTags),
    });
  };

  return (
    <section className="capture-workspace" aria-label="New item">
      <CaptureHeader mode={mode} setMode={setMode} />
      <form className="capture-form" onSubmit={submit}>
        <CaptureFields
          disabled={saving}
          mode={mode}
          setText={setText}
          setUrl={setUrl}
          text={text}
          url={url}
        />
        <TagCaptureField
          disabled={saving}
          onToggleTag={(tag) => setSelectedTags(toggleTag(selectedTags, tag))}
          popularTags={popularTags}
          selectedTags={selectedTags}
          setTagInput={setTagInput}
          tagInput={tagInput}
        />
        <CaptureActions canSave={canSave} message={message} saving={saving} />
      </form>
    </section>
  );
}

function CaptureHeader({
  mode,
  setMode,
}: {
  mode: CaptureMode;
  setMode: (mode: CaptureMode) => void;
}) {
  return (
    <div className="capture-header">
      <div>
        <p className="eyebrow">Quick capture</p>
        <h1>New item</h1>
      </div>
      <div className="capture-mode-switch" aria-label="Capture mode">
        <ModeButton active={mode === "text"} label="Text" onClick={() => setMode("text")} />
        <ModeButton active={mode === "link"} label="Link" onClick={() => setMode("link")} />
      </div>
    </div>
  );
}

function ModeButton({
  active,
  label,
  onClick,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      aria-pressed={active}
      className={active ? "mode-option active" : "mode-option"}
      onClick={onClick}
      type="button"
    >
      {label}
    </button>
  );
}

function CaptureFields({
  mode,
  text,
  url,
  disabled,
  setText,
  setUrl,
}: {
  mode: CaptureMode;
  text: string;
  url: string;
  disabled: boolean;
  setText: (value: string) => void;
  setUrl: (value: string) => void;
}) {
  if (mode === "text") {
    return <TextCaptureField disabled={disabled} setText={setText} text={text} />;
  }
  return <LinkCaptureField disabled={disabled} setUrl={setUrl} url={url} />;
}

function TextCaptureField({
  text,
  setText,
  disabled,
}: {
  text: string;
  setText: (value: string) => void;
  disabled: boolean;
}) {
  return (
    <label className="capture-field capture-field-large">
      Body
      <textarea
        disabled={disabled}
        name="plain_text"
        onChange={(event) => setText(event.currentTarget.value)}
        rows={10}
        value={text}
      />
    </label>
  );
}

function LinkCaptureField({
  url,
  setUrl,
  disabled,
}: {
  url: string;
  setUrl: (value: string) => void;
  disabled: boolean;
}) {
  return (
    <label className="capture-field">
      URL
      <input
        disabled={disabled}
        name="url"
        onChange={(event) => setUrl(event.currentTarget.value)}
        type="url"
        value={url}
      />
    </label>
  );
}

function TagCaptureField({
  popularTags,
  selectedTags,
  tagInput,
  disabled,
  onToggleTag,
  setTagInput,
}: {
  popularTags: TagCorpusEntry[];
  selectedTags: string[];
  tagInput: string;
  disabled: boolean;
  onToggleTag: (tag: string) => void;
  setTagInput: (value: string) => void;
}) {
  return (
    <div className="capture-tags">
      <div className="tag-options">
        {popularTags.map((tag) => (
          <button
            aria-pressed={selectedTags.includes(tag.display_name)}
            className={selectedTags.includes(tag.display_name) ? "tag-chip selected" : "tag-chip"}
            disabled={disabled}
            key={tag.id}
            onClick={() => onToggleTag(tag.display_name)}
            type="button"
          >
            {tag.display_name}
          </button>
        ))}
      </div>
      <label className="capture-field">
        Tags
        <input
          disabled={disabled}
          onChange={(event) => setTagInput(event.currentTarget.value)}
          value={tagInput}
        />
      </label>
    </div>
  );
}

function CaptureActions({
  saving,
  canSave,
  message,
}: {
  saving: boolean;
  canSave: boolean;
  message: SaveMessage;
}) {
  return (
    <div className="capture-actions">
      <button className="primary-action" disabled={saving || !canSave} type="submit">
        {saving ? "Saving" : "Save item"}
      </button>
      <span aria-live="polite" className={`save-message ${message.kind}`}>
        {message.text}
      </span>
    </div>
  );
}

async function saveCapture(options: {
  mode: CaptureMode;
  text: string;
  url: string;
  tagInput: string;
  selectedTags: string[];
  onCreateText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  onCreateLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
  setSaving: (saving: boolean) => void;
  setMessage: (message: SaveMessage) => void;
  clearDraft: () => void;
}) {
  if (!canSaveDraft(options.mode, options.text, options.url)) {
    options.setMessage({ kind: "error", text: emptyDraftMessage(options.mode) });
    return;
  }
  options.setSaving(true);
  options.setMessage({ kind: "idle", text: "Saving" });
  try {
    const outcome = await createCapture(options);
    options.clearDraft();
    options.setMessage({
      kind: "success",
      text: outcome.created ? "Saved to vault" : "Already in vault",
    });
  } catch (error) {
    options.setMessage({
      kind: "error",
      text: error instanceof Error ? `Save failed: ${error.message}` : "Save failed",
    });
  } finally {
    options.setSaving(false);
  }
}

async function createCapture(options: {
  mode: CaptureMode;
  text: string;
  url: string;
  tagInput: string;
  selectedTags: string[];
  onCreateText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  onCreateLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
}) {
  const tags = selectedTagValues(options.selectedTags, options.tagInput);
  if (options.mode === "text") {
    return options.onCreateText(await textRequest(options.text, tags));
  }
  return options.onCreateLink(linkRequest(options.url.trim(), tags));
}

async function textRequest(plainText: string, tags: string[]): Promise<CaptureTextRequest> {
  return {
    plain_text: plainText,
    html: null,
    source_app: "Bookmarker",
    source_device: await sourceDevice(),
    capture_method: "desktop_manual",
    tags,
    client_capture_id: randomId(),
  };
}

function linkRequest(url: string, tags: string[]): CaptureLinkRequest {
  return {
    url,
    tags,
    client_capture_id: randomId(),
  };
}

function canSaveDraft(mode: CaptureMode, text: string, url: string) {
  return mode === "text" ? text.trim().length > 0 : url.trim().length > 0;
}

function emptyDraftMessage(mode: CaptureMode) {
  return mode === "text" ? "Enter text before saving" : "Enter a URL before saving";
}

async function sourceDevice() {
  return (await desktopBridge()?.platform()) ?? null;
}

function selectedTagValues(selectedTags: string[], tagInput: string) {
  return dedupeTags([...selectedTags, ...tagInput.split(",")]);
}

function dedupeTags(tags: string[]) {
  const values = new Map<string, string>();
  for (const tag of tags.map((value) => value.trim()).filter(Boolean)) {
    values.set(tag.toLowerCase(), tag);
  }
  return [...values.values()];
}

function toggleTag(selectedTags: string[], tag: string) {
  return selectedTags.includes(tag)
    ? selectedTags.filter((selected) => selected !== tag)
    : [...selectedTags, tag];
}

function clearDraft(
  mode: CaptureMode,
  setText: (value: string) => void,
  setUrl: (value: string) => void,
  setTagInput: (value: string) => void,
  setSelectedTags: (tags: string[]) => void
) {
  if (mode === "text") {
    setText("");
  } else {
    setUrl("");
  }
  setTagInput("");
  setSelectedTags([]);
}

function randomId() {
  return globalThis.crypto?.randomUUID?.() ?? fallbackId();
}

let fallbackCounter = 0;

function fallbackId() {
  fallbackCounter += 1;
  return `capture-${Date.now()}-${fallbackCounter}`;
}
