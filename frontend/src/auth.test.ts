import { describe, expect, it } from "vitest";
import { createAuthClient, createDesktopCognitoStorage, type AuthAdapter } from "./auth";
import type { DesktopBridge } from "./desktopBridge";

describe("auth client", () => {
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

  it("desktop_cognito_storage_uses_the_electron_credential_bridge", () => {
    const values = new Map<string, string>();
    const storage = createDesktopCognitoStorage(fakeDesktopBridge(values));

    storage?.setItem("token", "saved-token");

    expect(storage?.getItem("token")).toBe("saved-token");
    storage?.removeItem("token");
    expect(storage?.getItem("token")).toBeNull();
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

function fakeDesktopBridge(values: Map<string, string>): DesktopBridge {
  return {
    readClipboardText: async () => "",
    writeClipboardText: async () => undefined,
    platform: async () => "test",
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
