type SaveState = {
  kind: "idle" | "success" | "error";
  text: string;
};

type CaptureMode = "text" | "link";

export function QuickCaptureActions({
  saving,
  canSave,
  message,
  mode,
  setMode,
}: {
  saving: boolean;
  canSave: boolean;
  message: SaveState;
  mode: CaptureMode;
  setMode: (mode: CaptureMode) => void;
}) {
  return (
    <div className="quick-capture-actions">
      <button className="quick-save-button" disabled={saving || !canSave} type="submit">
        {saving ? "Saving" : "Save"}
      </button>
      <span aria-live="polite" className={`save-message ${message.kind}`}>
        {message.text}
      </span>
      <ModeToggle disabled={saving} mode={mode} setMode={setMode} />
    </div>
  );
}

function ModeToggle({
  mode,
  disabled,
  setMode,
}: {
  mode: CaptureMode;
  disabled: boolean;
  setMode: (mode: CaptureMode) => void;
}) {
  return (
    <div aria-label="Item type" className="quick-mode-toggle" role="radiogroup">
      <ModeOption active={mode === "text"} disabled={disabled} mode="text" setMode={setMode} />
      <ModeOption active={mode === "link"} disabled={disabled} mode="link" setMode={setMode} />
    </div>
  );
}

function ModeOption({
  mode,
  active,
  disabled,
  setMode,
}: {
  mode: CaptureMode;
  active: boolean;
  disabled: boolean;
  setMode: (mode: CaptureMode) => void;
}) {
  return (
    <label className={active ? "quick-mode-option active" : "quick-mode-option"}>
      <input
        checked={active}
        disabled={disabled}
        name="quick-capture-mode"
        onChange={() => setMode(mode)}
        type="radio"
        value={mode}
      />
      <span>{mode === "text" ? "Text" : "Link"}</span>
    </label>
  );
}
