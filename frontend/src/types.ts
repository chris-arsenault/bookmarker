export type ArchiveStatus = "pending" | "succeeded" | "failed" | "not_applicable";
export type WatchStatus = "unwatched" | "watched";
export type InboxStatus = "unsorted" | "organized";
export type ItemKind = "url" | "text_snippet";

export type ApiDateTime =
  | string
  | number
  | number[]
  | Partial<
      Record<
        "seconds" | "secs" | "unix_timestamp" | "unixTimestamp" | "nanos" | "nanoseconds",
        number
      >
    >;

export type ItemTag = {
  id: string;
  display_name: string;
  normalized_name: string;
};

export type TagCorpusEntry = {
  id: string;
  display_name: string;
  normalized_name: string;
  usage_count: number;
};

export type LibraryItemSummary = {
  id: string;
  item_kind: ItemKind;
  url: ItemUrlSummary | null;
  text: ItemTextSummary | null;
  title: string | null;
  fetched_title: string | null;
  thumbnail_s3_key: string | null;
  author: string | null;
  platform: string | null;
  duration_seconds: number | null;
  archive_status: ArchiveStatus;
  watch_status: WatchStatus;
  inbox_status: InboxStatus;
  tags: ItemTag[];
  created_at: ApiDateTime;
};

export type ItemUrlSummary = {
  original_url: string;
  canonical_url: string | null;
  copy_url: string;
};

export type ItemTextSummary = {
  plain_text: string;
  preview: string;
  content_hash: string;
  html: string | null;
  source_app: string | null;
  source_device: string | null;
  capture_method: string;
};

export type LibraryItemDetail = {
  summary: LibraryItemSummary;
  notes: string;
};

export type CaptureItemOutcome = {
  item: LibraryItemDetail;
  created: boolean;
};

export type LibraryUpdates = {
  items: LibraryItemSummary[];
  deleted_item_ids: string[];
  tags: TagCorpusEntry[];
  cursor: ApiDateTime;
};

export type ListItemsFilters = Partial<{
  platform: string;
  tag: string;
  createdFrom: string;
  createdTo: string;
  archiveStatus: ArchiveStatus;
  watchStatus: WatchStatus;
  inboxStatus: InboxStatus;
  q: string;
}>;

export type ListItemUpdatesRequest = ListItemsFilters &
  Partial<{
    since: string;
    limit: number;
  }>;

export type UpdateItemRequest = Partial<{
  watch_status: WatchStatus;
  inbox_status: InboxStatus;
  notes: string;
  tags: string[];
}>;

export type CaptureTextRequest = {
  plain_text: string;
  title: string | null;
  html: string | null;
  source_app: string | null;
  source_device: string | null;
  capture_method: string | null;
  tags: string[];
  client_capture_id: string | null;
};

export type CaptureLinkRequest = {
  url: string;
  title: string | null;
  tags: string[];
  client_capture_id: string | null;
};

export type RenameTagRequest = {
  display_name: string;
};

export type MergeTagsRequest = {
  target_tag_id: string;
};
