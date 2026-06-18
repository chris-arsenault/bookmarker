// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { AuthClient, AuthState } from "./auth";
import { SignInPanel } from "./SignInPanel";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("SignInPanel MFA", () => {
  it("renders_software_token_setup_with_manual_secret_and_otpauth_link", () => {
    const authState: AuthState = {
      status: "mfa-setup",
      username: "chris",
      secretCode: "ABC123",
      otpAuthUri: "otpauth://totp/Linkdrop%3Achris?secret=ABC123&issuer=Linkdrop",
    };

    const html = renderToString(
      <SignInPanel authClient={fakeAuthClient()} authState={authState} />
    );

    expect(html).toContain("ABC123");
    expect(html).toContain("otpauth://totp/Linkdrop%3Achris?secret=ABC123&amp;issuer=Linkdrop");
    expect(html).toContain("Finish setup");
  });

  it("submits_software_token_mfa_code", async () => {
    const calls: string[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <SignInPanel
          authClient={fakeAuthClient({ confirmMfa: async (code) => calls.push(code) })}
          authState={{ status: "mfa-required", username: "chris" }}
        />
      );
    });

    setFieldValue(container, "code", "123456");
    await submitForm(container);

    expect(calls).toEqual(["123456"]);
    root.unmount();
    container.remove();
  });

  it("submits_software_token_setup_code", async () => {
    const calls: string[] = [];
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <SignInPanel
          authClient={fakeAuthClient({ verifyMfaSetup: async (code) => calls.push(code) })}
          authState={{
            status: "mfa-setup",
            username: "chris",
            secretCode: "ABC123",
            otpAuthUri: "otpauth://totp/Linkdrop%3Achris?secret=ABC123&issuer=Linkdrop",
          }}
        />
      );
    });

    setFieldValue(container, "code", "654321");
    await submitForm(container);

    expect(calls).toEqual(["654321"]);
    root.unmount();
    container.remove();
  });
});

function fakeAuthClient(overrides: Partial<AuthClient> = {}): AuthClient {
  return {
    getState: () => ({ status: "signed-out" }),
    subscribe: () => () => undefined,
    init: async () => ({ status: "signed-out" }),
    signIn: async () => undefined,
    confirmMfa: async () => undefined,
    verifyMfaSetup: async () => undefined,
    cancelChallenge: async () => undefined,
    logout: async () => undefined,
    getAccessToken: async () => undefined,
    ...overrides,
  };
}

function setFieldValue(container: HTMLElement, name: string, value: string) {
  const field = container.querySelector(`input[name="${name}"]`) as HTMLInputElement;
  field.value = value;
}

async function submitForm(container: HTMLElement) {
  const form = container.querySelector("form") as HTMLFormElement;
  await act(async () => {
    form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
  });
}
