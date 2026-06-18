import { CaptureWorkspace } from "./CaptureWorkspace";
import { FilterBar } from "./FilterBar";
import { ItemDetail } from "./ItemDetail";
import { LibraryFeed } from "./LibraryFeed";
import { createLibraryViewModel, type LibraryState, type LibraryViewModel } from "./libraryState";
import { platformOptions, tagOptions, type LibraryFilters } from "./libraryFilters";
import { TagManager } from "./TagManager";
import { AppBar, VaultRail } from "./VaultChrome";
import { config } from "./config";
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
    <div className="vault">
      <VaultRail
        filters={filters}
        itemCount={viewModel.items.length}
        onFiltersChange={onFiltersChange}
        tagCount={viewModel.tags.length}
      />
      <main className="workspace">
        <AppBar
          activeFilters={activeFilterCount(filters)}
          filters={filters}
          savedCount={viewModel.items.length}
        />
        <div className="workspace-body">
          <div className="primary-column">
            {onCreateText && onCreateLink ? (
              <CaptureWorkspace
                onCreateLink={onCreateLink}
                onCreateText={onCreateText}
                tags={viewModel.tags}
              />
            ) : null}
            <FilterBar
              filters={filters}
              platforms={platformOptions(viewModel.items)}
              tags={tagOptions(viewModel.tags)}
              onFiltersChange={onFiltersChange}
            />
            <div className="feed-scroll">
              <LibraryFeed
                items={viewModel.items}
                selectedItemId={viewModel.selectedItem?.id ?? null}
                thumbnailUrls={thumbnailUrls}
                onCopyItem={onCopyLink}
                onSelectItem={onSelectItem}
              />
              <details className="corpus">
                <summary>Tag corpus</summary>
                <TagManager
                  tags={viewModel.tags}
                  onMergeTags={onMergeTags}
                  onRenameTag={onRenameTag}
                />
              </details>
            </div>
          </div>
          <ItemDetail
            availableTags={viewModel.tags}
            detail={viewModel.selectedDetail}
            onCopyLink={onCopyLink}
            onDeleteItem={onDeleteItem}
            onOpenSource={onOpenSource}
            onUpdateItem={onUpdateItem}
          />
        </div>
      </main>
    </div>
  );
}

function InactiveLibraryView({ status, message }: { status: string; message: string | null }) {
  return (
    <main className="app-shell app-shell-centered">
      <section className="empty-state">
        <p className="eyebrow">{config.productName}</p>
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
