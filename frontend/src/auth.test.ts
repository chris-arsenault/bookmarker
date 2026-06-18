import { describe, expect, it } from "vitest";
import { createAuthClient, type AuthAdapter } from "./auth";
import {
  cognitoSessionFromStoredAuthSession,
  createAuthSessionStore,
  type StoredAuthSession,
} from "./authSessionStore";
import type { DesktopBridge } from "./desktopBridge";

describe("auth client session flow", () => {
  it("auth_client_reports_signed_out_without_session", async () => {
    const adapter: AuthAdapter = {
      getSession: async () => null,
      refreshSession: async () => null,
      signIn: async () => {
        throw new Error("not used");
      },
      confirmMfa: async () => {
        throw new Error("not used");
      },
      verifyMfaSetup: async () => {
        throw new Error("not used");
      },
      cancelChallenge: () => undefined,
      signOut: () => undefined,
    };

    const auth = createAuthClient({ adapter });

    await expect(auth.init()).resolves.toEqual({ status: "signed-out" });
    expect(auth.getState()).toEqual({ status: "signed-out" });
  });

  it("auth_client_exposes_software_token_mfa_challenge", async () => {
    const auth = createAuthClient({
      adapter: fakeAdapter({
        signIn: async () => ({ status: "mfa-required", username: "chris" }),
        confirmMfa: async () => ({ status: "signed-in", session: fakeSession }),
      }),
    });

    await auth.signIn("chris", "password");
    expect(auth.getState()).toEqual({ status: "mfa-required", username: "chris" });

    await auth.confirmMfa("123456");
    expect(auth.getState()).toEqual({
      status: "signed-in",
      user: { subject: "sub", email: "chris@example.test", username: "chris" },
    });
  });

  it("auth_client_keeps_the_mfa_session_available_before_storage_round_trip", async () => {
    const auth = createAuthClient({
      adapter: fakeAdapter({
        getSession: async () => null,
        signIn: async () => ({ status: "mfa-required", username: "chris" }),
        confirmMfa: async () => ({ status: "signed-in", session: fakeSession }),
      }),
    });

    await auth.signIn("chris", "password");
    await auth.confirmMfa("123456");

    await expect(auth.getAccessToken()).resolves.toBe("access-token");
    expect(auth.getState()).toMatchObject({ status: "signed-in" });
  });
});

describe("auth client mfa setup flow", () => {
  it("auth_client_exposes_software_token_setup_challenge", async () => {
    const auth = createAuthClient({
      adapter: fakeAdapter({
        signIn: async () => ({
          status: "mfa-setup",
          username: "chris",
          secretCode: "ABC123",
          otpAuthUri: "otpauth://totp/Linkdrop%3Achris?secret=ABC123&issuer=Linkdrop",
        }),
        verifyMfaSetup: async () => ({ status: "signed-in", session: fakeSession }),
      }),
    });

    await auth.signIn("chris", "password");
    expect(auth.getState()).toEqual({
      status: "mfa-setup",
      username: "chris",
      secretCode: "ABC123",
      otpAuthUri: "otpauth://totp/Linkdrop%3Achris?secret=ABC123&issuer=Linkdrop",
    });

    await auth.verifyMfaSetup("123456");
    expect(auth.getState()).toMatchObject({ status: "signed-in" });
  });
});

describe("auth session store", () => {
  it("auth_session_store_persists_the_app_owned_token_bundle_through_the_desktop_bridge", () => {
    const values = new Map<string, string>();
    const store = createAuthSessionStore(fakeDesktopBridge(values));
    const session = storedSession();

    store.save(session);

    expect(store.load()).toEqual(session);
    store.clear();
    expect(store.load()).toBeNull();
  });

  it("auth_session_store_rebuilds_cognito_sessions_from_persisted_tokens", () => {
    const stored = storedSession();
    const session = cognitoSessionFromStoredAuthSession(stored);

    expect(session.isValid()).toBe(true);
    expect(session.getAccessToken().getJwtToken()).toBe(stored.accessToken);
    expect(session.getRefreshToken().getToken()).toBe(stored.refreshToken);
  });
});

function fakeAdapter(overrides: Partial<AuthAdapter>): AuthAdapter {
  return {
    getSession: async () => null,
    refreshSession: async () => null,
    signIn: async () => {
      throw new Error("not used");
    },
    confirmMfa: async () => {
      throw new Error("not used");
    },
    verifyMfaSetup: async () => {
      throw new Error("not used");
    },
    cancelChallenge: () => undefined,
    signOut: () => undefined,
    ...overrides,
  };
}

const fakeSession = {
  getAccessToken: () => ({ getJwtToken: () => "access-token" }),
  getIdToken: () => ({
    decodePayload: () => ({
      sub: "sub",
      email: "chris@example.test",
      "cognito:username": "chris",
    }),
  }),
  isValid: () => true,
};

function storedSession(): StoredAuthSession {
  const now = Math.floor(Date.now() / 1000);
  return {
    username: "chris",
    idToken: jwt({
      sub: "sub",
      email: "chris@example.test",
      "cognito:username": "chris",
      exp: now + 3600,
      iat: now - 60,
    }),
    accessToken: jwt({
      sub: "sub",
      scope: "openid",
      exp: now + 3600,
      iat: now - 60,
    }),
    refreshToken: "refresh-token",
  };
}

function jwt(payload: Record<string, unknown>) {
  return [base64Url({ alg: "none", typ: "JWT" }), base64Url(payload), "signature"].join(".");
}

function base64Url(value: Record<string, unknown>) {
  const encoded = btoa(JSON.stringify(value)).replaceAll("+", "-").replaceAll("/", "_");
  const paddingIndex = encoded.indexOf("=");
  return paddingIndex === -1 ? encoded : encoded.slice(0, paddingIndex);
}

function fakeDesktopBridge(values: Map<string, string>): DesktopBridge {
  return {
    readClipboardText: async () => "",
    writeClipboardText: async () => undefined,
    platform: async () => "test",
    credentialPath: async () => "/home/test/.config/Bookmarker/cognito-session.json",
    credentialGet: (key) => values.get(key) ?? null,
    credentialSet: (key, value) => {
      values.set(key, value);
    },
    credentialRemove: (key) => {
      values.delete(key);
    },
    credentialClear: () => {
      values.clear();
    },
  };
}
