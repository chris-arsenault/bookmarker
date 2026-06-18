import { app, safeStorage } from "electron";
import fs from "node:fs";
import path from "node:path";

type StoredCredentials = Record<string, string>;

const encryptedPrefix = "safe:v1:";
const plainPrefix = "plain:v1:";

export type DesktopCredentialStore = {
  path: string;
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  clear: () => void;
};

export function createDesktopCredentialStore(): DesktopCredentialStore {
  const storePath = path.join(app.getPath("userData"), "cognito-session.json");
  const legacyStorePaths = legacyCredentialStorePaths(storePath);
  return {
    path: storePath,
    getItem: (key) => readDecodedValue(storePath, legacyStorePaths, key),
    setItem: (key, value) => writeStoreValue(storePath, key, value),
    removeItem: (key) => removeStoreValue(storePath, key),
    clear: () => clearStore(storePath),
  };
}

function readDecodedValue(storePath: string, legacyStorePaths: string[], key: string) {
  for (const candidatePath of [storePath, ...legacyStorePaths]) {
    const value = decodeValue(readStore(candidatePath)[key]);
    if (value !== null) {
      return value;
    }
  }
  return null;
}

function legacyCredentialStorePaths(storePath: string) {
  return ["bookmarker-desktop", "bookmarker-frontend"]
    .map((name) => path.join(app.getPath("appData"), name, "cognito-session.json"))
    .filter((legacyPath) => legacyPath !== storePath);
}

function writeStoreValue(storePath: string, key: string, value: string) {
  const store = readStore(storePath);
  store[key] = encodeValue(value);
  writeStore(storePath, store);
}

function removeStoreValue(storePath: string, key: string) {
  const store = readStore(storePath);
  delete store[key];
  writeStore(storePath, store);
}

function clearStore(storePath: string) {
  try {
    fs.rmSync(storePath, { force: true });
  } catch {
    writeStore(storePath, {});
  }
}

function readStore(storePath: string): StoredCredentials {
  try {
    const parsed = JSON.parse(fs.readFileSync(storePath, "utf8")) as unknown;
    return isStoredCredentials(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

function writeStore(storePath: string, store: StoredCredentials) {
  fs.mkdirSync(path.dirname(storePath), { recursive: true });
  fs.writeFileSync(storePath, `${JSON.stringify(store, null, 2)}\n`, { mode: 0o600 });
}

function encodeValue(value: string) {
  if (safeStorage.isEncryptionAvailable()) {
    return `${encryptedPrefix}${safeStorage.encryptString(value).toString("base64")}`;
  }
  return `${plainPrefix}${value}`;
}

function decodeValue(value: string | undefined) {
  if (!value) {
    return null;
  }
  try {
    if (value.startsWith(encryptedPrefix)) {
      return safeStorage.decryptString(Buffer.from(value.slice(encryptedPrefix.length), "base64"));
    }
    if (value.startsWith(plainPrefix)) {
      return value.slice(plainPrefix.length);
    }
    return value;
  } catch {
    return null;
  }
}

function isStoredCredentials(value: unknown): value is StoredCredentials {
  return (
    typeof value === "object" &&
    value !== null &&
    Object.values(value).every((entry) => typeof entry === "string")
  );
}
