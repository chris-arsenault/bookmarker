import { useCallback, useMemo, useState } from "react";
import { FilterBar } from "./FilterBar";
import { ItemDetail } from "./ItemDetail";
import { LibraryFeed } from "./LibraryFeed";
import { useLibraryActions } from "./LibraryActionsContext";
import { createLibraryViewModel, type LibraryViewModel } from "./libraryState";
import { platformOptions, tagOptions, type LibraryFilters } from "./libraryFilters";
import { QuickTextCapture } from "./QuickTextCapture";
import { TagManager } from "./TagManager";
import { AppBar, VaultRail } from "./VaultChrome";
import { config } from "./config";
import type { LibraryItemDetail, LibraryItemSummary, TagCorpusEntry } from "./types";
import type { LibrarySnapshot } from "./useLibraryController";

type LibraryViewProps = {
  snapshot: LibrarySnapshot;
};

type ReadyLibraryViewProps = {
  snapshot: LibrarySnapshot;
  viewModel: LibraryViewModel;
};

export function LibraryView({ snapshot }: LibraryViewProps) {
  const viewModel = useMemo(() => createLibraryViewModel(snapshot.state), [snapshot.state]);
  if (viewModel.status !== "ready") {
    return <InactiveLibraryView status={viewModel.status} message={viewModel.errorMessage} />;
  }
  return <ReadyLibraryView snapshot={snapshot} viewModel={viewModel} />;
}

function ReadyLibraryView({ snapshot, viewModel }: ReadyLibraryViewProps) {
  const { changeFilters: applyFilters, closeDetail, selectItem } = useLibraryActions();
  const { filters, thumbnailUrls } = snapshot;
  const detailModal = useDetailModal(viewModel.selectedDetail, selectItem, closeDetail);
  const changeFilters = useCallback(
    (nextFilters: LibraryFilters) => {
      detailModal.close();
      applyFilters(nextFilters);
    },
    [applyFilters, detailModal]
  );

  return (
    <div className="vault">
      <VaultRail
        filters={filters}
        itemCount={viewModel.items.length}
        onFiltersChange={changeFilters}
        tagCount={viewModel.tags.length}
      >
        <RailCapture tags={viewModel.tags} />
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
              filters={filters}
              items={viewModel.items}
              onFiltersChange={changeFilters}
              tagCorpus={viewModel.tags}
            />
            <FeedPanel
              thumbnailUrls={thumbnailUrls}
              viewModel={viewModel}
              onSelectItem={detailModal.open}
            />
          </div>
          <ItemDetail
            availableTags={viewModel.tags}
            detail={detailModal.detail}
            onClose={detailModal.close}
          />
        </div>
      </main>
    </div>
  );
}

function RailCapture({ tags }: { tags: TagCorpusEntry[] }) {
  const { createLink, createText } = useLibraryActions();
  return <QuickTextCapture tags={tags} onCreateLink={createLink} onCreateText={createText} />;
}

function FeedPanel({
  viewModel,
  thumbnailUrls,
  onSelectItem,
}: {
  viewModel: LibraryViewModel;
  thumbnailUrls: Record<string, string>;
  onSelectItem: (itemId: string) => void;
}) {
  const { copyItem } = useLibraryActions();
  return (
    <div className="feed-scroll">
      <LibraryFeed
        items={viewModel.items}
        selectedItemId={viewModel.selectedItem?.id ?? null}
        thumbnailUrls={thumbnailUrls}
        onCopyItem={copyItem}
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
  const detail = selectedDetail?.summary.id === modalItemId ? selectedDetail : null;
  const open = useCallback(
    (itemId: string) => {
      setModalItemId(itemId);
      onSelectItem(itemId);
    },
    [onSelectItem]
  );
  const close = useCallback(() => {
    setModalItemId(null);
    onCloseDetail();
  }, [onCloseDetail]);
  return useMemo(() => ({ close, detail, open }), [close, detail, open]);
}

function SearchHeader({
  filters,
  items,
  tagCorpus,
  onFiltersChange,
}: {
  filters: LibraryFilters;
  items: LibraryItemSummary[];
  tagCorpus: TagCorpusEntry[];
  onFiltersChange: (filters: LibraryFilters) => void;
}) {
  const { mergeTags, renameTag } = useLibraryActions();
  return (
    <details className="search-drawer">
      <summary>
        <span>Search and filters</span>
        <small>{activeFilterCount(filters)} active</small>
      </summary>
      <FilterBar
        filters={filters}
        platforms={platformOptions(items)}
        tags={tagOptions(tagCorpus)}
        onFiltersChange={onFiltersChange}
      />
      <details className="corpus">
        <summary>Tag corpus</summary>
        <TagManager tags={tagCorpus} onMergeTags={mergeTags} onRenameTag={renameTag} />
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
