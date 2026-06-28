import { useEffect, useMemo, useState } from "react";
import { ApiClient } from "./api";
import { createAuthClient, type AuthClient, type AuthState } from "./auth";
import { LibraryViewShell } from "./LibraryViewShell";
import { SignInPanel } from "./SignInPanel";

const authClient = createAuthClient();

export function App() {
  const apiClient = useMemo(
    () =>
      new ApiClient({
        getAccessToken: (request) => authClient.getAccessToken(request),
      }),
    []
  );
  return <AuthenticatedApp apiClient={apiClient} authClient={authClient} />;
}

function AuthenticatedApp({
  apiClient,
  authClient,
}: {
  apiClient: ApiClient;
  authClient: AuthClient;
}) {
  const [authState, setAuthState] = useState<AuthState>(authClient.getState());

  useEffect(() => {
    const unsubscribe = authClient.subscribe(setAuthState);
    authClient.init().catch(() => {});
    return unsubscribe;
  }, [authClient]);

  if (authState.status !== "signed-in") {
    return <SignInPanel authClient={authClient} authState={authState} />;
  }
  return <LibraryViewShell apiClient={apiClient} />;
}
