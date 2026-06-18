export type DesktopBridge = {
  readClipboardText: () => Promise<string>;
  writeClipboardText: (value: string) => Promise<void>;
  platform: () => Promise<string>;
  credentialGet: (key: string) => string | null;
  credentialSet: (key: string, value: string) => void;
  credentialRemove: (key: string) => void;
  credentialClear: () => void;
};

declare global {
  interface Window {
    bookmarkerDesktop: DesktopBridge | null;
  }
}

export function desktopBridge() {
  if (typeof window === "undefined") {
    return null;
  }
  return window.bookmarkerDesktop || null;
}
