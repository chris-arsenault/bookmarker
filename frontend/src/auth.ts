import { createCognitoAdapter } from "./cognitoAuthAdapter";
import { config as runtimeConfig } from "./config";

export type AuthState =
  | { status: "loading" }
  | { status: "signed-out" }
  | { status: "signed-in"; user: AuthUser }
  | { status: "mfa-required"; username: string }
  | { status: "mfa-setup"; username: string; secretCode: string; otpAuthUri: string }
  | { status: "error"; message: string };

export type AuthUser = {
  subject: string | null;
  email: string | null;
  username: string | null;
};

export type AccessTokenRequest = Partial<{
  forceRefresh: boolean;
}>;

export type SessionLike = {
  getAccessToken: () => { getJwtToken: () => string };
  getIdToken: () => { decodePayload: () => Record<string, unknown> };
  isValid: () => boolean;
};

export type AuthAdapter = {
  getSession: () => Promise<SessionLike | null>;
  refreshSession: () => Promise<SessionLike | null>;
  signIn: (username: string, password: string) => Promise<AuthAdapterResult>;
  confirmMfa: (code: string) => Promise<AuthAdapterResult>;
  verifyMfaSetup: (code: string) => Promise<AuthAdapterResult>;
  cancelChallenge: () => void;
  signOut: () => void;
};

export type AuthAdapterResult =
  | { status: "signed-in"; session: SessionLike }
  | { status: "mfa-required"; username: string }
  | { status: "mfa-setup"; username: string; secretCode: string; otpAuthUri: string };

export type AuthClient = {
  getState: () => AuthState;
  subscribe: (listener: (state: AuthState) => void) => () => void;
  init: () => Promise<AuthState>;
  signIn: (username: string, password: string) => Promise<void>;
  confirmMfa: (code: string) => Promise<void>;
  verifyMfaSetup: (code: string) => Promise<void>;
  cancelChallenge: () => Promise<void>;
  logout: () => Promise<void>;
  getAccessToken: (request?: AccessTokenRequest) => Promise<string | undefined>;
};

export type CreateAuthClientOptions = Partial<{
  adapter: AuthAdapter;
  config: typeof runtimeConfig;
}>;

class BrowserAuthClient implements AuthClient {
  private state: AuthState = { status: "loading" };
  private session: SessionLike | null = null;
  private readonly listeners = new Set<(state: AuthState) => void>();
  private readonly adapter: AuthAdapter;

  constructor(adapter: AuthAdapter) {
    this.adapter = adapter;
  }

  getState() {
    return this.state;
  }

  subscribe(listener: (state: AuthState) => void) {
    this.listeners.add(listener);
    listener(this.state);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async init() {
    this.setState({ status: "loading" });
    try {
      const session = await this.adapter.getSession();
      this.applySession(session);
    } catch (error) {
      this.setState({ status: "error", message: authErrorMessage(error) });
    }
    return this.state;
  }

  async signIn(username: string, password: string) {
    const result = await this.adapter.signIn(username, password);
    this.applyAdapterResult(result);
  }

  async confirmMfa(code: string) {
    const result = await this.adapter.confirmMfa(code);
    this.applyAdapterResult(result);
  }

  async verifyMfaSetup(code: string) {
    const result = await this.adapter.verifyMfaSetup(code);
    this.applyAdapterResult(result);
  }

  async cancelChallenge() {
    this.adapter.cancelChallenge();
    this.setState({ status: "signed-out" });
  }

  async logout() {
    this.adapter.signOut();
    this.session = null;
    this.setState({ status: "signed-out" });
  }

  async getAccessToken(request: AccessTokenRequest = {}) {
    if (this.state.status !== "signed-in") {
      return undefined;
    }
    try {
      const session = await this.accessTokenSession(request);
      this.applySession(session);
      return this.state.status === "signed-in"
        ? session?.getAccessToken().getJwtToken()
        : undefined;
    } catch {
      this.session = null;
      this.setState({ status: "signed-out" });
      return undefined;
    }
  }

  private async accessTokenSession(request: AccessTokenRequest) {
    if (request.forceRefresh) {
      return this.adapter.refreshSession();
    }
    if (this.session?.isValid()) {
      return this.session;
    }
    return this.adapter.getSession();
  }

  private applyAdapterResult(result: AuthAdapterResult) {
    if (result.status === "signed-in") {
      this.applySession(result.session);
      return;
    }
    this.session = null;
    this.setState(result);
  }

  private applySession(session: SessionLike | null) {
    this.session = session?.isValid() ? session : null;
    this.setState(stateFromSession(this.session));
  }

  private setState(state: AuthState) {
    this.state = state;
    this.listeners.forEach((listener) => listener(state));
  }
}

export function createAuthClient(options: CreateAuthClientOptions = {}): AuthClient {
  if (options.adapter) {
    return new BrowserAuthClient(options.adapter);
  }
  if (options.config) {
    return new BrowserAuthClient(createCognitoAdapter(options.config));
  }
  return new BrowserAuthClient(createCognitoAdapter(runtimeConfig));
}

function stateFromSession(session: SessionLike | null): AuthState {
  if (!session?.isValid()) {
    return { status: "signed-out" };
  }
  return { status: "signed-in", user: userFromSession(session) };
}

function userFromSession(session: SessionLike): AuthUser {
  const payload = session.getIdToken().decodePayload();
  return {
    subject: stringClaim(payload.sub),
    email: stringClaim(payload.email),
    username: stringClaim(payload["cognito:username"]),
  };
}

function stringClaim(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function authErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "authentication failed";
}
