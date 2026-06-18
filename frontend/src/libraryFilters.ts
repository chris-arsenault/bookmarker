import type {
  ArchiveStatus,
  InboxStatus,
  LibraryItemSummary,
  ListItemsFilters,
  TagCorpusEntry,
  WatchStatus,
} from "./types";

export type LibraryFilters = Partial<{
  platform: string;
  tag: string;
  createdFrom: string;
  createdTo: string;
  archiveStatus: ArchiveStatus | "all";
  watchStatus: WatchStatus | "all";
  inboxStatus: InboxStatus | "all";
  q: string;
}>;

export type FilterOption = {
  value: string;
  label: string;
  count: number;
};

export function libraryFiltersToApiFilters(filters: LibraryFilters): ListItemsFilters {
  return {
    ...stringFilter("platform", filters.platform),
    ...stringFilter("tag", filters.tag),
    ...stringFilter("createdFrom", filters.createdFrom),
    ...stringFilter("createdTo", filters.createdTo),
    ...statusFilter("archiveStatus", filters.archiveStatus),
    ...statusFilter("watchStatus", filters.watchStatus),
    ...statusFilter("inboxStatus", filters.inboxStatus),
    ...stringFilter("q", filters.q),
  };
}

export function platformOptions(items: LibraryItemSummary[]): FilterOption[] {
  const counts = new Map<string, number>();
  items.forEach((item) => {
    const platform = cleanText(item.platform);
    if (platform) {
      counts.set(platform, (counts.get(platform) ?? 0) + 1);
    }
  });
  return [...counts.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([platform, count]) => ({ value: platform, label: platform, count }));
}

export function tagOptions(tags: TagCorpusEntry[]): FilterOption[] {
  return tags.map((tag) => ({
    value: tag.normalized_name,
    label: tag.display_name,
    count: tag.usage_count,
  }));
}

function stringFilter<K extends keyof ListItemsFilters>(key: K, value: string | null | undefined) {
  const clean = cleanText(value);
  return clean ? { [key]: clean } : {};
}

function statusFilter<K extends keyof ListItemsFilters>(key: K, value: string | null | undefined) {
  return value && value !== "all" ? { [key]: value } : {};
}

function cleanText(value: string | null | undefined) {
  const clean = value?.trim();
  return clean && clean.length > 0 ? clean : null;
}
