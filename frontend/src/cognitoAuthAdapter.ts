import {
  AuthenticationDetails,
  CognitoUser,
  CognitoUserPool,
  type ICognitoStorage,
  type CognitoUserSession,
} from "amazon-cognito-identity-js";
import {
  cognitoSessionFromStoredAuthSession,
  createAuthSessionStore,
  refreshTokenFromStoredAuthSession,
  storedAuthSessionFromCognitoSession,
  type AuthSessionStore,
  type StoredAuthSession,
} from "./authSessionStore";
import type { AuthAdapter, AuthAdapterResult } from "./auth";

type CognitoAdapterConfig = {
  cognitoUserPoolId: string;
  cognitoClientId: string;
  productName: string;
};

type CognitoAuthAdapterResult =
  | { status: "signed-in"; session: CognitoUserSession }
  | { status: "mfa-required"; username: string }
  | { status: "mfa-setup"; username: string; secretCode: string; otpAuthUri: string };

export function createCognitoAdapter(config: CognitoAdapterConfig): AuthAdapter {
  const userPool = new CognitoUserPool({
    UserPoolId: requiredConfig(config.cognitoUserPoolId, "cognitoUserPoolId"),
    ClientId: requiredConfig(config.cognitoClientId, "cognitoClientId"),
    Storage: createVolatileCognitoStorage(),
  });
  return new CognitoAuthAdapter(
    userPool,
    requiredConfig(config.productName, "productName"),
    createAuthSessionStore()
  );
}

class CognitoAuthAdapter implements AuthAdapter {
  private pendingUser: CognitoUser | null = null;

  constructor(
    private readonly userPool: CognitoUserPool,
    private readonly productName: string,
    private readonly sessionStore: AuthSessionStore
  ) {}

  getSession() {
    return this.storedSession();
  }

  refreshSession() {
    return this.storedSession({ forceRefresh: true });
  }

  signIn(username: string, password: string) {
    const user = new CognitoUser({ Username: username, Pool: this.userPool });
    const details = new AuthenticationDetails({ Username: username, Password: password });
    return signInWithUserPool(user, details, (pendingUser) =>
      this.setPendingUser(pendingUser)
    ).then((result) => this.persistResult(username, result));
  }

  confirmMfa(code: string) {
    const user = this.requirePendingUser();
    return confirmSoftwareTokenMfa(user, code).then((result) =>
      this.clearPendingAfterSuccess(user, result)
    );
  }

  verifyMfaSetup(code: string) {
    const user = this.requirePendingUser();
    return verifySoftwareToken(user, code, this.productName).then((result) =>
      this.clearPendingAfterSuccess(user, result)
    );
  }

  cancelChallenge() {
    this.pendingUser?.signOut();
    this.pendingUser = null;
  }

  signOut() {
    this.pendingUser = null;
    this.sessionStore.clear();
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

  private clearPendingAfterSuccess(user: CognitoUser, result: CognitoAuthAdapterResult) {
    this.persistResult(user.getUsername(), result);
    if (result.status === "signed-in") {
      this.pendingUser = null;
    }
    return result;
  }

  private persistResult(username: string, result: CognitoAuthAdapterResult): AuthAdapterResult {
    if (result.status === "signed-in") {
      this.saveSession(username, result.session);
    }
    return result;
  }

  private saveSession(username: string, session: CognitoUserSession) {
    this.sessionStore.save(storedAuthSessionFromCognitoSession(username, session));
  }

  private async storedSession({ forceRefresh = false } = {}) {
    const stored = this.sessionStore.load();
    if (!stored) {
      return null;
    }
    try {
      const session = cognitoSessionFromStoredAuthSession(stored);
      if (!forceRefresh && session.isValid()) {
        return session;
      }
      return await this.refreshStoredSession(stored);
    } catch {
      this.sessionStore.clear();
      return null;
    }
  }

  private refreshStoredSession(stored: StoredAuthSession) {
    const user = new CognitoUser({ Username: stored.username, Pool: this.userPool });
    return new Promise<CognitoUserSession | null>((resolve, reject) => {
      user.refreshSession(refreshTokenFromStoredAuthSession(stored), (refreshError, session) => {
        if (refreshError) {
          reject(refreshError);
          return;
        }
        if (!session) {
          resolve(null);
          return;
        }
        this.saveSession(stored.username, session);
        resolve(session);
      });
    });
  }
}

function signInWithUserPool(
  user: CognitoUser,
  details: AuthenticationDetails,
  setPendingUser: (user: CognitoUser) => string
): Promise<CognitoAuthAdapterResult> {
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

function confirmSoftwareTokenMfa(
  user: CognitoUser,
  code: string
): Promise<CognitoAuthAdapterResult> {
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

function selectSoftwareTokenMfa(user: CognitoUser): Promise<CognitoAuthAdapterResult> {
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
): Promise<CognitoAuthAdapterResult> {
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

function requiredConfig(value: string, name: string): string {
  if (value.trim().length === 0) {
    throw new Error(`missing runtime config: ${name}`);
  }
  return value;
}

function createVolatileCognitoStorage(): ICognitoStorage {
  const values = new Map<string, string>();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    removeItem: (key) => {
      values.delete(key);
    },
    clear: () => {
      values.clear();
    },
  };
}
