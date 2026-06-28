import { createContext, useContext } from "react";
import type { LibraryFilters } from "./libraryFilters";
import type {
  CaptureItemOutcome,
  CaptureLinkRequest,
  CaptureTextRequest,
  LibraryItemDetail,
  LibraryItemSummary,
  MergeTagsRequest,
  RenameTagRequest,
  TagCorpusEntry,
  UpdateItemRequest,
} from "./types";

export type LibraryActions = {
  changeFilters: (filters: LibraryFilters) => void;
  closeDetail: () => void;
  copyItem: (item: LibraryItemSummary) => void;
  createLink: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
  createText: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  deleteItem: (itemId: string) => Promise<void>;
  mergeTags: (sourceTagId: string, request: MergeTagsRequest) => Promise<TagCorpusEntry[]>;
  openSource: (url: string) => void;
  renameTag: (tagId: string, request: RenameTagRequest) => Promise<TagCorpusEntry[]>;
  selectItem: (itemId: string) => void;
  updateItem: (itemId: string, request: UpdateItemRequest) => Promise<LibraryItemDetail>;
};

export const LibraryActionsContext = createContext<LibraryActions | null>(null);

export function useLibraryActions() {
  const actions = useContext(LibraryActionsContext);
  if (!actions) {
    throw new Error("Library actions are not available");
  }
  return actions;
}
