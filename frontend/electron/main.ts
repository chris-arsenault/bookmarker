import { app, BrowserWindow, clipboard, ipcMain, Menu, nativeImage, Tray } from "electron";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createDesktopCredentialStore } from "./credentialStore.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const productName = "Bookmarker";

let mainWindow: BrowserWindow | null = null;
let hudWindow: BrowserWindow | null = null;
let activeTray: Tray | null = null;

app.setName(productName);
app.setPath("userData", path.join(app.getPath("appData"), productName));
app.setAppUserModelId("io.ahara.bookmarker");

app.whenReady().then(() => {
  registerIpc();
  mainWindow = createMainWindow();
  activeTray = createTray();
});

app.on("before-quit", () => {
  activeTray?.destroy();
});

app.on("window-all-closed", () => {});

app.on("activate", () => {
  showMainWindow();
});

function createMainWindow() {
  const window = new BrowserWindow({
    width: 1180,
    height: 820,
    title: "Bookmarker Vault",
    webPreferences: webPreferences(),
  });
  loadFrontend(window);
  window.on("close", (event) => {
    event.preventDefault();
    window.hide();
  });
  return window;
}

function createHudWindow() {
  const window = new BrowserWindow({
    width: 420,
    height: 640,
    title: "Bookmarker HUD",
    alwaysOnTop: true,
    skipTaskbar: true,
    webPreferences: webPreferences(),
  });
  loadFrontend(window, "view=hud");
  window.on("close", (event) => {
    event.preventDefault();
    window.hide();
  });
  return window;
}

function webPreferences() {
  return {
    contextIsolation: true,
    preload: path.join(__dirname, "preload.js"),
    sandbox: false,
  };
}

function loadFrontend(window: BrowserWindow, search?: string) {
  const desktopUrl = desktopUrlOverride();
  if (desktopUrl) {
    const query = search ? `?${search}` : "";
    window.loadURL(`${desktopUrl}${query}`);
    return;
  }
  window.loadFile(path.join(__dirname, "../dist/index.html"), { search });
}

function desktopUrlOverride() {
  const environmentUrl = process.env.BOOKMARKER_DESKTOP_URL?.trim();
  if (environmentUrl) {
    return environmentUrl;
  }
  return undefined;
}

function createTray() {
  const appTray = new Tray(trayIcon());
  appTray.setToolTip("Bookmarker Vault");
  appTray.setContextMenu(
    Menu.buildFromTemplate([
      { label: "Show Vault", click: showMainWindow },
      { label: "Toggle HUD", click: toggleHudWindow },
      { type: "separator" },
      { label: "Quit", click: () => app.exit(0) },
    ])
  );
  appTray.on("click", showMainWindow);
  return appTray;
}

function trayIcon() {
  const icon = nativeImage.createFromPath(path.join(__dirname, "../dist/favicon-32x32.png"));
  return icon.isEmpty() ? nativeImage.createEmpty() : icon;
}

function showMainWindow() {
  if (!mainWindow) {
    mainWindow = createMainWindow();
  }
  mainWindow.show();
  mainWindow.focus();
}

function toggleHudWindow() {
  if (!hudWindow) {
    hudWindow = createHudWindow();
  }
  if (hudWindow.isVisible()) {
    hudWindow.hide();
    return;
  }
  hudWindow.show();
}

function registerIpc() {
  const credentialStore = createDesktopCredentialStore();
  ipcMain.handle("desktop:read-clipboard-text", () => clipboard.readText());
  ipcMain.handle("desktop:write-clipboard-text", (_event, value: string) => {
    clipboard.writeText(value);
  });
  ipcMain.handle("desktop:platform", () => process.platform);
  ipcMain.handle("desktop:credential-path", () => credentialStore.path);
  ipcMain.on("desktop:credential-get", (event, key: string) => {
    event.returnValue = credentialStore.getItem(key);
  });
  ipcMain.on("desktop:credential-set", (event, key: string, value: string) => {
    credentialStore.setItem(key, value);
    event.returnValue = null;
  });
  ipcMain.on("desktop:credential-remove", (event, key: string) => {
    credentialStore.removeItem(key);
    event.returnValue = null;
  });
  ipcMain.on("desktop:credential-clear", (event) => {
    credentialStore.clear();
    event.returnValue = null;
  });
}
