import type { ReactNode } from "react";
import { ImageAccessContext, type ImageAccessLoader } from "./ImageAccessContext";

export function ImageAccessProvider({
  loadImageAccess,
  children,
}: {
  loadImageAccess: ImageAccessLoader;
  children: ReactNode;
}) {
  return (
    <ImageAccessContext.Provider value={loadImageAccess}>{children}</ImageAccessContext.Provider>
  );
}
