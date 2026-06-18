import { useState } from "react";
import { desktopBridge } from "./desktopBridge";

export function SaveErrorModal({ message, onClose }: { message: string; onClose: () => void }) {
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");

  const copy = async () => {
    try {
      await writeClipboardText(message);
      setCopyState("copied");
    } catch {
      setCopyState("failed");
    }
  };

  return (
    <div className="modal-backdrop">
      <section
        aria-labelledby="save-error-title"
        aria-modal="true"
        className="error-modal"
        role="dialog"
      >
        <button aria-label="Close error" className="modal-close" onClick={onClose} type="button">
          &times;
        </button>
        <div>
          <h2 id="save-error-title">Save failed</h2>
          <p>The item was not saved.</p>
        </div>
        <pre>{message}</pre>
        <div className="error-modal-actions">
          <button onClick={copy} type="button">
            {copyState === "copied" ? "Copied" : "Copy"}
          </button>
          {copyState === "failed" ? <span>Copy failed</span> : null}
        </div>
      </section>
    </div>
  );
}

async function writeClipboardText(value: string) {
  const bridge = desktopBridge();
  if (bridge) {
    await bridge.writeClipboardText(value);
    return;
  }
  await navigator.clipboard.writeText(value);
}
