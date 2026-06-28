import { createContext, useContext } from "react";
import type { ImageAccessTarget } from "./types";

export type ImageAccessLoader = (itemId: string) => Promise<ImageAccessTarget>;

export const ImageAccessContext = createContext<ImageAccessLoader | null>(null);

export function useImageAccessLoader() {
  const loadImageAccess = useContext(ImageAccessContext);
  if (!loadImageAccess) {
    throw new Error("Image access loader is not available");
  }
  return loadImageAccess;
}
