import {
  AuthenticationDetails,
  CognitoUser,
  CognitoUserPool,
  type ICognitoStorage,
  type CognitoUserSession,
} from "amazon-cognito-identity-js";
import { config as runtimeConfig } from "./config";
import { desktopBridge, type DesktopBridge } from "./desktopBridge";

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
      this.setState(stateFromSession(session));
    } catch (error) {
      this.setState({ status: "error", message: authErrorMessage(error) });
    }
    return this.state;
  }

  async signIn(username: string, password: string) {
    const result = await this.adapter.signIn(username, password);
    this.setState(stateFromAdapterResult(result));
  }

  async confirmMfa(code: string) {
    const result = await this.adapter.confirmMfa(code);
    this.setState(stateFromAdapterResult(result));
  }

  async verifyMfaSetup(code: string) {
    const result = await this.adapter.verifyMfaSetup(code);
    this.setState(stateFromAdapterResult(result));
  }

  async cancelChallenge() {
    this.adapter.cancelChallenge();
    this.setState({ status: "signed-out" });
  }

  async logout() {
    this.adapter.signOut();
    this.setState({ status: "signed-out" });
  }

  async getAccessToken(request: AccessTokenRequest = {}) {
    if (this.state.status !== "signed-in") {
      return undefined;
    }
    try {
      const session = request.forceRefresh
        ? await this.adapter.refreshSession()
        : await this.adapter.getSession();
      const state = stateFromSession(session);
      this.setState(state);
      return state.status === "signed-in" ? session?.getAccessToken().getJwtToken() : undefined;
    } catch {
      this.setState({ status: "signed-out" });
      return undefined;
    }
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

function createCognitoAdapter(config: typeof runtimeConfig): AuthAdapter {
  const storage = createDesktopCognitoStorage();
  const userPool = new CognitoUserPool({
    UserPoolId: requiredConfig(config.cognitoUserPoolId, "cognitoUserPoolId"),
    ClientId: requiredConfig(config.cognitoClientId, "cognitoClientId"),
    ...(storage ? { Storage: storage } : {}),
  });
  return new CognitoAuthAdapter(userPool, requiredConfig(config.productName, "productName"));
}

export function createDesktopCognitoStorage(
  bridge: DesktopBridge | null = desktopBridge()
): ICognitoStorage | undefined {
  if (!bridge) {
    return undefined;
  }
  return {
    getItem: bridge.credentialGet,
    setItem: bridge.credentialSet,
    removeItem: bridge.credentialRemove,
    clear: bridge.credentialClear,
  };
}

class CognitoAuthAdapter implements AuthAdapter {
  private pendingUser: CognitoUser | null = null;

  constructor(
    private readonly userPool: CognitoUserPool,
    private readonly productName: string
  ) {}

  getSession() {
    return currentSession(this.userPool);
  }

  refreshSession() {
    return refreshCurrentSession(this.userPool);
  }

  signIn(username: string, password: string) {
    const user = new CognitoUser({ Username: username, Pool: this.userPool });
    const details = new AuthenticationDetails({ Username: username, Password: password });
    return signInWithUserPool(user, details, (pendingUser) => this.setPendingUser(pendingUser));
  }

  confirmMfa(code: string) {
    return confirmSoftwareTokenMfa(this.requirePendingUser(), code).then((result) =>
      this.clearPendingAfterSuccess(result)
    );
  }

  verifyMfaSetup(code: string) {
    return verifySoftwareToken(this.requirePendingUser(), code, this.productName).then((result) =>
      this.clearPendingAfterSuccess(result)
    );
  }

  cancelChallenge() {
    this.pendingUser?.signOut();
    this.pendingUser = null;
  }

  signOut() {
    this.pendingUser = null;
    this.userPool.getCurrentUser()?.signOut();
  }

  private setPendingUser(user: CognitoUser) {
    this.pendingUser = user;
    return this.productName;
  }

  private requirePendingUser() {
    if (!this.pendingUser) {
      throw new Error("missing MFA challenge");
    }
    return this.pendingUser;
  }

  private clearPendingAfterSuccess(result: AuthAdapterResult) {
    if (result.status === "signed-in") {
      this.pendingUser = null;
    }
    return result;
  }
}

function currentSession(userPool: CognitoUserPool): Promise<CognitoUserSession | null> {
  const user = userPool.getCurrentUser();
  if (!user) {
    return Promise.resolve(null);
  }
  return new Promise((resolve, reject) => {
    user.getSession((error: Error | null, session: CognitoUserSession | null) => {
      if (error) {
        reject(error);
        return;
      }
      resolve(session);
    });
  });
}

function refreshCurrentSession(userPool: CognitoUserPool): Promise<CognitoUserSession | null> {
  const user = userPool.getCurrentUser();
  if (!user) {
    return Promise.resolve(null);
  }
  return new Promise((resolve, reject) => {
    user.getSession((error: Error | null, session: CognitoUserSession | null) => {
      if (error || !session) {
        reject(error ?? new Error("missing session"));
        return;
      }
      user.refreshSession(session.getRefreshToken(), (refreshError, refreshedSession) => {
        if (refreshError) {
          reject(refreshError);
          return;
        }
        resolve(refreshedSession);
      });
    });
  });
}

function signInWithUserPool(
  user: CognitoUser,
  details: AuthenticationDetails,
  setPendingUser: (user: CognitoUser) => string
): Promise<AuthAdapterResult> {
  const username = user.getUsername();
  return new Promise((resolve, reject) => {
    user.authenticateUser(details, {
      onSuccess: (session) => resolve({ status: "signed-in", session }),
      onFailure: reject,
      newPasswordRequired: () => reject(new Error("new password required")),
      mfaRequired: () => reject(new Error("SMS MFA is not supported")),
      selectMFAType: () => selectSoftwareTokenMfa(user).then(resolve, reject),
      totpRequired: () => {
        setPendingUser(user);
        resolve({ status: "mfa-required", username });
      },
      mfaSetup: () => {
        const productName = setPendingUser(user);
        associateSoftwareToken(user).then(
          (secretCode) =>
            resolve({
              status: "mfa-setup",
              username,
              secretCode,
              otpAuthUri: totpUri(productName, username, secretCode),
            }),
          reject
        );
      },
    });
  });
}

function confirmSoftwareTokenMfa(user: CognitoUser, code: string): Promise<AuthAdapterResult> {
  return new Promise((resolve, reject) => {
    user.sendMFACode(
      code,
      {
        onSuccess: (session) => resolve({ status: "signed-in", session }),
        onFailure: reject,
      },
      "SOFTWARE_TOKEN_MFA"
    );
  });
}

function selectSoftwareTokenMfa(user: CognitoUser): Promise<AuthAdapterResult> {
  return new Promise((resolve, reject) => {
    user.sendMFASelectionAnswer("SOFTWARE_TOKEN_MFA", {
      onSuccess: (session) => resolve({ status: "signed-in", session }),
      onFailure: reject,
      mfaRequired: () => reject(new Error("SMS MFA is not supported")),
      totpRequired: () => resolve({ status: "mfa-required", username: user.getUsername() }),
    });
  });
}

function associateSoftwareToken(user: CognitoUser): Promise<string> {
  return new Promise((resolve, reject) => {
    user.associateSoftwareToken({
      associateSecretCode: resolve,
      onFailure: reject,
    });
  });
}

function verifySoftwareToken(
  user: CognitoUser,
  code: string,
  productName: string
): Promise<AuthAdapterResult> {
  return new Promise((resolve, reject) => {
    user.verifySoftwareToken(code, productName, {
      onSuccess: (session) => resolve({ status: "signed-in", session }),
      onFailure: reject,
    });
  });
}

function totpUri(productName: string, username: string, secretCode: string) {
  const issuer = encodeURIComponent(productName);
  const account = encodeURIComponent(`${productName}:${username}`);
  return `otpauth://totp/${account}?secret=${encodeURIComponent(secretCode)}&issuer=${issuer}`;
}

function stateFromAdapterResult(result: AuthAdapterResult): AuthState {
  if (result.status === "signed-in") {
    return stateFromSession(result.session);
  }
  return result;
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

function requiredConfig(value: string, name: string): string {
  if (value.trim().length === 0) {
    throw new Error(`missing runtime config: ${name}`);
  }
  return value;
}
