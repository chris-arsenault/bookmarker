import { useEffect, useMemo, useState } from "react";
import { ApiClient } from "./api";
import { createAuthClient, type AuthState } from "./auth";
import { desktopBridge } from "./desktopBridge";
import { formatDate } from "./dateDisplay";
import { itemCopyValue, itemFeedMeta, itemTitle } from "./itemDisplay";
import type { LibraryItemSummary } from "./types";

const authClient = createAuthClient();

export function DesktopHudApp() {
  const apiClient = useMemo(
    () =>
      new ApiClient({
        getAccessToken: (request) => authClient.getAccessToken(request),
      }),
    []
  );
  const [authState, setAuthState] = useState<AuthState>(authClient.getState());
  const [items, setItems] = useState<LibraryItemSummary[]>([]);

  useEffect(() => {
    const unsubscribe = authClient.subscribe(setAuthState);
    authClient.init().catch(() => {});
    return unsubscribe;
  }, []);

  useEffect(() => {
    if (authState.status !== "signed-in") {
      return;
    }
    apiClient
      .listItems({ inboxStatus: "unsorted" })
      .then((nextItems) => setItems(nextItems.slice(0, 8)))
      .catch(() => setItems([]));
  }, [apiClient, authState.status]);

  if (authState.status !== "signed-in") {
    return (
      <main className="desktop-hud">
        <p>Sign in from the vault window</p>
      </main>
    );
  }
  return (
    <main className="desktop-hud">
      <h1>HUD</h1>
      <div className="desktop-hud-list">
        {items.map((item) => (
          <button
            className="desktop-hud-item"
            key={item.id}
            onClick={() => copyHudItem(item)}
            type="button"
          >
            <span>{itemTitle(item)}</span>
            <small>{itemFeedMeta(item, formatDate(item.created_at))}</small>
          </button>
        ))}
      </div>
    </main>
  );
}

function copyHudItem(item: LibraryItemSummary) {
  const value = itemCopyValue(item);
  const bridge = desktopBridge();
  if (bridge) {
    bridge.writeClipboardText(value).catch(() => {});
    return;
  }
  navigator.clipboard.writeText(value).catch(() => {});
}
