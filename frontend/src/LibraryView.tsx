import { useState } from "react";
import { FilterBar } from "./FilterBar";
import { ItemDetail } from "./ItemDetail";
import { LibraryFeed } from "./LibraryFeed";
import { createLibraryViewModel, type LibraryState, type LibraryViewModel } from "./libraryState";
import { platformOptions, tagOptions, type LibraryFilters } from "./libraryFilters";
import { QuickTextCapture } from "./QuickTextCapture";
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
  onCloseDetail: () => void;
  onFiltersChange: (filters: LibraryFilters) => void;
  onCopyLink: (item: LibraryItemSummary) => void;
  onOpenSource: (url: string) => void;
  onLoadImage?: (itemId: string) => Promise<Blob>;
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

function ReadyLibraryView(props: ReadyLibraryViewProps) {
  const { filters, viewModel } = props;
  const detailModal = useDetailModal(
    viewModel.selectedDetail,
    props.onSelectItem,
    props.onCloseDetail
  );
  const changeFilters = (nextFilters: LibraryFilters) => {
    detailModal.close();
    props.onFiltersChange(nextFilters);
  };

  return (
    <div className="vault">
      <VaultRail
        filters={filters}
        itemCount={viewModel.items.length}
        onFiltersChange={changeFilters}
        tagCount={viewModel.tags.length}
      >
        <RailCapture
          tags={viewModel.tags}
          onCreateLink={props.onCreateLink}
          onCreateText={props.onCreateText}
        />
      </VaultRail>
      <main className="workspace">
        <AppBar
          activeFilters={activeFilterCount(filters)}
          filters={filters}
          savedCount={viewModel.items.length}
        />
        <div className="workspace-body">
          <div className="primary-column">
            <SearchHeader
              activeFilters={activeFilterCount(filters)}
              filters={filters}
              onFiltersChange={changeFilters}
              onMergeTags={props.onMergeTags}
              onRenameTag={props.onRenameTag}
              platforms={platformOptions(viewModel.items)}
              tags={tagOptions(viewModel.tags)}
              tagCorpus={viewModel.tags}
            />
            <FeedPanel
              thumbnailUrls={props.thumbnailUrls}
              viewModel={viewModel}
              onCopyLink={props.onCopyLink}
              onSelectItem={detailModal.open}
            />
          </div>
          <ItemDetail
            availableTags={viewModel.tags}
            detail={detailModal.detail}
            onClose={detailModal.close}
            onCopyLink={props.onCopyLink}
            onDeleteItem={props.onDeleteItem}
            onLoadImage={props.onLoadImage}
            onOpenSource={props.onOpenSource}
            onUpdateItem={props.onUpdateItem}
          />
        </div>
      </main>
    </div>
  );
}

function RailCapture({
  tags,
  onCreateText,
  onCreateLink,
}: {
  tags: TagCorpusEntry[];
  onCreateText: ReadyLibraryViewProps["onCreateText"];
  onCreateLink: ReadyLibraryViewProps["onCreateLink"];
}) {
  if (!onCreateText || !onCreateLink) {
    return null;
  }
  return <QuickTextCapture tags={tags} onCreateLink={onCreateLink} onCreateText={onCreateText} />;
}

function FeedPanel({
  viewModel,
  thumbnailUrls,
  onCopyLink,
  onSelectItem,
}: {
  viewModel: LibraryViewModel;
  thumbnailUrls: Record<string, string>;
  onCopyLink: (item: LibraryItemSummary) => void;
  onSelectItem: (itemId: string) => void;
}) {
  return (
    <div className="feed-scroll">
      <LibraryFeed
        items={viewModel.items}
        selectedItemId={viewModel.selectedItem?.id ?? null}
        thumbnailUrls={thumbnailUrls}
        onCopyItem={onCopyLink}
        onSelectItem={onSelectItem}
      />
    </div>
  );
}

function useDetailModal(
  selectedDetail: LibraryItemDetail | null,
  onSelectItem: (itemId: string) => void,
  onCloseDetail: () => void
) {
  const [modalItemId, setModalItemId] = useState<string | null>(null);
  return {
    detail: selectedDetail?.summary.id === modalItemId ? selectedDetail : null,
    open: (itemId: string) => {
      setModalItemId(itemId);
      onSelectItem(itemId);
    },
    close: () => {
      setModalItemId(null);
      onCloseDetail();
    },
  };
}

function SearchHeader({
  filters,
  platforms,
  tags,
  tagCorpus,
  activeFilters,
  onFiltersChange,
  onRenameTag,
  onMergeTags,
}: {
  filters: LibraryFilters;
  platforms: ReturnType<typeof platformOptions>;
  tags: ReturnType<typeof tagOptions>;
  tagCorpus: TagCorpusEntry[];
  activeFilters: number;
  onFiltersChange: (filters: LibraryFilters) => void;
  onRenameTag: (tagId: string, request: RenameTagRequest) => Promise<TagCorpusEntry[]>;
  onMergeTags: (sourceTagId: string, request: MergeTagsRequest) => Promise<TagCorpusEntry[]>;
}) {
  return (
    <details className="search-drawer">
      <summary>
        <span>Search and filters</span>
        <small>{activeFilters} active</small>
      </summary>
      <FilterBar
        filters={filters}
        platforms={platforms}
        tags={tags}
        onFiltersChange={onFiltersChange}
      />
      <details className="corpus">
        <summary>Tag corpus</summary>
        <TagManager tags={tagCorpus} onMergeTags={onMergeTags} onRenameTag={onRenameTag} />
      </details>
    </details>
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
