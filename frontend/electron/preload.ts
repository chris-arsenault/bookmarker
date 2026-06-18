import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("bookmarkerDesktop", {
  readClipboardText: () => ipcRenderer.invoke("desktop:read-clipboard-text"),
  writeClipboardText: (value: string) => ipcRenderer.invoke("desktop:write-clipboard-text", value),
  platform: () => ipcRenderer.invoke("desktop:platform"),
  credentialGet: (key: string) => ipcRenderer.sendSync("desktop:credential-get", key),
  credentialSet: (key: string, value: string) =>
    ipcRenderer.sendSync("desktop:credential-set", key, value),
  credentialRemove: (key: string) => ipcRenderer.sendSync("desktop:credential-remove", key),
  credentialClear: () => ipcRenderer.sendSync("desktop:credential-clear"),
});
