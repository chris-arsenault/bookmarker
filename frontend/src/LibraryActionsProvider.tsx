import type { ReactNode } from "react";
import { LibraryActionsContext, type LibraryActions } from "./LibraryActionsContext";

export function LibraryActionsProvider({
  actions,
  children,
}: {
  actions: LibraryActions;
  children: ReactNode;
}) {
  return (
    <LibraryActionsContext.Provider value={actions}>{children}</LibraryActionsContext.Provider>
  );
}
