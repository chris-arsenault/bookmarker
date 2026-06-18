import { app, safeStorage } from "electron";
import fs from "node:fs";
import path from "node:path";

type StoredCredentials = Record<string, string>;

const encryptedPrefix = "safe:v1:";
const plainPrefix = "plain:v1:";

export type DesktopCredentialStore = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  clear: () => void;
};

export function createDesktopCredentialStore(): DesktopCredentialStore {
  const storePath = path.join(app.getPath("userData"), "cognito-session.json");
  return {
    getItem: (key) => decodeValue(readStore(storePath)[key]),
    setItem: (key, value) => writeStoreValue(storePath, key, value),
    removeItem: (key) => removeStoreValue(storePath, key),
    clear: () => clearStore(storePath),
  };
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
  if (value.startsWith(encryptedPrefix)) {
    return safeStorage.decryptString(Buffer.from(value.slice(encryptedPrefix.length), "base64"));
  }
  if (value.startsWith(plainPrefix)) {
    return value.slice(plainPrefix.length);
  }
  return value;
}

function isStoredCredentials(value: unknown): value is StoredCredentials {
  return (
    typeof value === "object" &&
    value !== null &&
    Object.values(value).every((entry) => typeof entry === "string")
  );
}
