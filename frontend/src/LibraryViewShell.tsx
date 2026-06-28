import type { ApiClient } from "./api";
import { ImageAccessProvider } from "./ImageAccessProvider";
import { LibraryActionsProvider } from "./LibraryActionsProvider";
import { LibraryView } from "./LibraryView";
import { useLibraryController } from "./useLibraryController";

type LibraryViewShellProps = {
  apiClient: ApiClient;
};

export function LibraryViewShell({ apiClient }: LibraryViewShellProps) {
  const { actions, loadImageAccess, snapshot } = useLibraryController(apiClient);

  return (
    <LibraryActionsProvider actions={actions}>
      <ImageAccessProvider loadImageAccess={loadImageAccess}>
        <LibraryView snapshot={snapshot} />
      </ImageAccessProvider>
    </LibraryActionsProvider>
  );
}
