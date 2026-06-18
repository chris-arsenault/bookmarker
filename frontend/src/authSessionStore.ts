import {
  CognitoAccessToken,
  CognitoIdToken,
  CognitoRefreshToken,
  CognitoUserSession,
} from "amazon-cognito-identity-js";
import { desktopBridge, type DesktopBridge } from "./desktopBridge";

const authSessionKey = "bookmarker.auth.session.v1";

export type StoredAuthSession = {
  username: string;
  idToken: string;
  accessToken: string;
  refreshToken: string;
};

export type AuthSessionStore = {
  load: () => StoredAuthSession | null;
  save: (session: StoredAuthSession) => void;
  clear: () => void;
};

export function createAuthSessionStore(
  bridge: DesktopBridge | null = desktopBridge()
): AuthSessionStore {
  if (bridge) {
    return bridgeSessionStore(bridge);
  }
  return browserSessionStore();
}

export function storedAuthSessionFromCognitoSession(
  username: string,
  session: CognitoUserSession
): StoredAuthSession {
  return {
    username,
    idToken: session.getIdToken().getJwtToken(),
    accessToken: session.getAccessToken().getJwtToken(),
    refreshToken: session.getRefreshToken().getToken(),
  };
}

export function cognitoSessionFromStoredAuthSession(stored: StoredAuthSession) {
  return new CognitoUserSession({
    IdToken: new CognitoIdToken({ IdToken: stored.idToken }),
    AccessToken: new CognitoAccessToken({ AccessToken: stored.accessToken }),
    RefreshToken: new CognitoRefreshToken({ RefreshToken: stored.refreshToken }),
  });
}

export function refreshTokenFromStoredAuthSession(stored: StoredAuthSession) {
  return new CognitoRefreshToken({ RefreshToken: stored.refreshToken });
}

function bridgeSessionStore(bridge: DesktopBridge): AuthSessionStore {
  return {
    load: () => parseStoredSession(bridge.credentialGet(authSessionKey)),
    save: (session) => bridge.credentialSet(authSessionKey, JSON.stringify(session)),
    clear: () => bridge.credentialRemove(authSessionKey),
  };
}

function browserSessionStore(): AuthSessionStore {
  return {
    load: () => parseStoredSession(readBrowserSession()),
    save: (session) => writeBrowserSession(JSON.stringify(session)),
    clear: () => removeBrowserSession(),
  };
}

function readBrowserSession() {
  try {
    return globalThis.localStorage?.getItem(authSessionKey) ?? null;
  } catch {
    return null;
  }
}

function writeBrowserSession(value: string) {
  try {
    globalThis.localStorage?.setItem(authSessionKey, value);
  } catch {
    // Persistence is best effort; the live session remains in memory.
  }
}

function removeBrowserSession() {
  try {
    globalThis.localStorage?.removeItem(authSessionKey);
  } catch {
    // Nothing to clear.
  }
}

function parseStoredSession(value: string | null): StoredAuthSession | null {
  if (!value) {
    return null;
  }
  try {
    const parsed = JSON.parse(value) as unknown;
    return isStoredAuthSession(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function isStoredAuthSession(value: unknown): value is StoredAuthSession {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as Partial<StoredAuthSession>;
  return (
    isPresentString(candidate.username) &&
    isPresentString(candidate.idToken) &&
    isPresentString(candidate.accessToken) &&
    isPresentString(candidate.refreshToken)
  );
}

function isPresentString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}
