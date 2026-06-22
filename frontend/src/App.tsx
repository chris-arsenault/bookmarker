import { useCallback, useEffect, useMemo, useState } from "react";
import { ApiClient } from "./api";
import { createAuthClient, type AuthClient, type AuthState } from "./auth";
import { loadLibrary } from "./libraryActions";
import type { LibraryFilters } from "./libraryFilters";
import type { LibraryState } from "./libraryState";
import { LibraryViewShell } from "./LibraryViewShell";
import { SignInPanel } from "./SignInPanel";
import { useLibraryUpdatePoller } from "./useLibraryUpdatePoller";

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
  const [libraryState, setLibraryState] = useState<LibraryState>({ status: "loading" });
  const [filters, setFilters] = useState<LibraryFilters>({});
  const [thumbnailUrls, setThumbnailUrls] = useState<Record<string, string>>({});
  const [updatesCursor, setUpdatesCursor] = useState<string | null>(null);
  const refreshLibrary = useCallback(
    () => loadLibrary(apiClient, filters, setLibraryState, setThumbnailUrls, setUpdatesCursor),
    [apiClient, filters]
  );

  useEffect(() => {
    const unsubscribe = authClient.subscribe(setAuthState);
    authClient.init().catch(() => {});
    return unsubscribe;
  }, [authClient]);

  useEffect(() => {
    if (authState.status !== "signed-in") {
      return;
    }
    refreshLibrary().catch(() => {});
  }, [authState.status, refreshLibrary]);

  useLibraryUpdatePoller({
    apiClient,
    authStatus: authState.status,
    filters,
    libraryState,
    updatesCursor,
    setUpdatesCursor,
    setLibraryState,
    setThumbnailUrls,
  });

  if (authState.status !== "signed-in") {
    return <SignInPanel authClient={authClient} authState={authState} />;
  }
  return (
    <LibraryViewShell
      apiClient={apiClient}
      filters={filters}
      libraryState={libraryState}
      setFilters={setFilters}
      setLibraryState={setLibraryState}
      setThumbnailUrls={setThumbnailUrls}
      setUpdatesCursor={setUpdatesCursor}
      thumbnailUrls={thumbnailUrls}
    />
  );
}
