import { CaptureWorkspace } from "./CaptureWorkspace";
import { FilterBar } from "./FilterBar";
import { ItemDetail } from "./ItemDetail";
import { LibraryFeed } from "./LibraryFeed";
import { createLibraryViewModel, type LibraryState, type LibraryViewModel } from "./libraryState";
import { platformOptions, tagOptions, type LibraryFilters } from "./libraryFilters";
import { TagManager } from "./TagManager";
import type {
  LibraryItemDetail,
  LibraryItemSummary,
  CaptureItemOutcome,
  CaptureLinkRequest,
  CaptureTextRequest,
  MergeTagsRequest,
  RenameTagRequest,
  TagCorpusEntry,
  UpdateItemRequest,
} from "./types";

type LibraryViewProps = {
  state: LibraryState;
  filters: LibraryFilters;
  thumbnailUrls: Record<string, string>;
  onSelectItem: (itemId: string) => void;
  onFiltersChange: (filters: LibraryFilters) => void;
  onCopyLink: (item: LibraryItemSummary) => void;
  onOpenSource: (url: string) => void;
  onUpdateItem: (itemId: string, request: UpdateItemRequest) => Promise<LibraryItemDetail>;
  onCreateText?: (request: CaptureTextRequest) => Promise<CaptureItemOutcome>;
  onCreateLink?: (request: CaptureLinkRequest) => Promise<CaptureItemOutcome>;
  onDeleteItem?: (itemId: string) => Promise<void>;
  onRenameTag: (tagId: string, request: RenameTagRequest) => Promise<TagCorpusEntry[]>;
  onMergeTags: (sourceTagId: string, request: MergeTagsRequest) => Promise<TagCorpusEntry[]>;
};

type ReadyLibraryViewProps = Omit<LibraryViewProps, "state"> & {
  viewModel: LibraryViewModel;
};

export function LibraryView(props: LibraryViewProps) {
  const viewModel = createLibraryViewModel(props.state);
  if (viewModel.status !== "ready") {
    return <InactiveLibraryView status={viewModel.status} message={viewModel.errorMessage} />;
  }
  return <ReadyLibraryView {...props} viewModel={viewModel} />;
}

function ReadyLibraryView({
  filters,
  thumbnailUrls,
  viewModel,
  onSelectItem,
  onFiltersChange,
  onCopyLink,
  onOpenSource,
  onUpdateItem,
  onCreateText,
  onCreateLink,
  onDeleteItem,
  onRenameTag,
  onMergeTags,
}: ReadyLibraryViewProps) {
  return (
    <main className="app-shell">
      {onCreateText && onCreateLink ? (
        <CaptureWorkspace
          onCreateLink={onCreateLink}
          onCreateText={onCreateText}
          tags={viewModel.tags}
        />
      ) : null}
      <header className="topbar">
        <div>
          <p className="eyebrow">Bookmarker</p>
          <h1>Vault</h1>
        </div>
        <p className="summary-line">
          {viewModel.items.length} saved · {activeFilterCount(filters)} active filters
        </p>
      </header>
      <FilterBar
        filters={filters}
        platforms={platformOptions(viewModel.items)}
        tags={tagOptions(viewModel.tags)}
        onFiltersChange={onFiltersChange}
      />
      <TagManager tags={viewModel.tags} onMergeTags={onMergeTags} onRenameTag={onRenameTag} />
      <section className="library-layout">
        <LibraryFeed
          items={viewModel.items}
          selectedItemId={viewModel.selectedItem?.id ?? null}
          thumbnailUrls={thumbnailUrls}
          onCopyItem={onCopyLink}
          onSelectItem={onSelectItem}
        />
        <ItemDetail
          availableTags={viewModel.tags}
          detail={viewModel.selectedDetail}
          onCopyLink={onCopyLink}
          onDeleteItem={onDeleteItem}
          onOpenSource={onOpenSource}
          onUpdateItem={onUpdateItem}
        />
      </section>
    </main>
  );
}

function InactiveLibraryView({ status, message }: { status: string; message: string | null }) {
  return (
    <main className="app-shell app-shell-centered">
      <section className="empty-state">
        <p className="eyebrow">Linkdrop</p>
        <h1>{status === "signed-out" ? "Sign in" : "Vault"}</h1>
        <p>{message ?? inactiveMessage(status)}</p>
      </section>
    </main>
  );
}

function inactiveMessage(status: string) {
  if (status === "loading") {
    return "Loading saved items";
  }
  if (status === "signed-out") {
    return "Authentication required";
  }
  return "No saved items match this view";
}

function activeFilterCount(filters: LibraryFilters) {
  return Object.values(filters).filter((value) => value && value !== "all").length;
}
