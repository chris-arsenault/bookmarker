import { useState, type FormEvent } from "react";
import type { AuthClient, AuthState } from "./auth";
import { config } from "./config";

export function SignInPanel({
  authClient,
  authState,
}: {
  authClient: AuthClient;
  authState: AuthState;
}) {
  const [message, setMessage] = useState("");
  const displayMessage = message || stateMessage(authState);

  return (
    <main className="app-shell app-shell-centered">
      <section className="signin-panel" aria-labelledby="signin-title">
        <p className="eyebrow">{config.productName}</p>
        <h1 id="signin-title">{config.productName}</h1>
        {authContent(authState, authClient, setMessage)}
        {displayMessage ? <p className="form-error">{displayMessage}</p> : null}
      </section>
    </main>
  );
}

function authContent(
  authState: AuthState,
  authClient: AuthClient,
  setMessage: (message: string) => void
) {
  if (authState.status === "loading") {
    return <p className="signin-note">Checking session...</p>;
  }
  if (authState.status === "mfa-required") {
    return <MfaCodeForm authClient={authClient} setMessage={setMessage} />;
  }
  if (authState.status === "mfa-setup") {
    return <MfaSetupForm authClient={authClient} authState={authState} setMessage={setMessage} />;
  }
  return <PasswordForm authClient={authClient} setMessage={setMessage} />;
}

function PasswordForm({
  authClient,
  setMessage,
}: {
  authClient: AuthClient;
  setMessage: (message: string) => void;
}) {
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    await runAuthAction(
      () => authClient.signIn(String(form.get("username")), String(form.get("password"))),
      setMessage,
      "sign in failed"
    );
  };
  return (
    <form className="signin-form" onSubmit={submit}>
      <label>
        Username
        <input name="username" autoComplete="username" />
      </label>
      <label>
        Password
        <input name="password" type="password" autoComplete="current-password" />
      </label>
      <button type="submit">Sign in</button>
    </form>
  );
}

function MfaCodeForm({
  authClient,
  setMessage,
}: {
  authClient: AuthClient;
  setMessage: (message: string) => void;
}) {
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    await runAuthAction(
      () => authClient.confirmMfa(String(form.get("code"))),
      setMessage,
      "MFA confirmation failed"
    );
  };
  return (
    <form className="signin-form" onSubmit={submit}>
      <label>
        Authenticator code
        <input name="code" inputMode="numeric" autoComplete="one-time-code" />
      </label>
      <button type="submit">Verify code</button>
      <button
        className="secondary-action"
        type="button"
        onClick={() => authClient.cancelChallenge()}
      >
        Cancel
      </button>
    </form>
  );
}

function MfaSetupForm({
  authClient,
  authState,
  setMessage,
}: {
  authClient: AuthClient;
  authState: Extract<AuthState, { status: "mfa-setup" }>;
  setMessage: (message: string) => void;
}) {
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    await runAuthAction(
      () => authClient.verifyMfaSetup(String(form.get("code"))),
      setMessage,
      "MFA setup failed"
    );
  };
  return (
    <form className="signin-form" onSubmit={submit}>
      <p className="signin-note">Add this setup key to your authenticator app.</p>
      <code className="setup-code">{authState.secretCode}</code>
      <a className="otpauth-link" href={authState.otpAuthUri}>
        Open authenticator setup
      </a>
      <label>
        Authenticator code
        <input name="code" inputMode="numeric" autoComplete="one-time-code" />
      </label>
      <button type="submit">Finish setup</button>
      <button
        className="secondary-action"
        type="button"
        onClick={() => authClient.cancelChallenge()}
      >
        Cancel
      </button>
    </form>
  );
}

async function runAuthAction(
  action: () => Promise<void>,
  setMessage: (message: string) => void,
  fallback: string
) {
  setMessage("");
  try {
    await action();
  } catch (error) {
    setMessage(error instanceof Error ? error.message : fallback);
  }
}

function stateMessage(authState: AuthState) {
  return authState.status === "error" ? authState.message : "";
}
