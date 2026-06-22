use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{ArchiveStatus, ImageUploadStatus, InboxStatus, ItemKind, WatchStatus};
use crate::library::{
    ItemImageSummary, ItemTag, ItemTextSummary, ItemUrlSummary, LibraryItemDetail,
    LibraryItemSummary, TagCorpusEntry,
};

#[derive(sqlx::FromRow)]
pub(crate) struct ItemRow {
    pub(crate) id: Uuid,
    pub(crate) item_kind: String,
    pub(crate) original_url: Option<String>,
    pub(crate) canonical_url: Option<String>,
    pub(crate) plain_text: Option<String>,
    pub(crate) html_content: Option<String>,
    pub(crate) content_hash: Option<String>,
    pub(crate) text_source_app: Option<String>,
    pub(crate) text_source_device: Option<String>,
    pub(crate) text_capture_method: Option<String>,
    pub(crate) image_s3_key: Option<String>,
    pub(crate) image_content_type: Option<String>,
    pub(crate) image_original_filename: Option<String>,
    pub(crate) image_byte_size: Option<i64>,
    pub(crate) image_upload_status: Option<String>,
    pub(crate) image_source_app: Option<String>,
    pub(crate) image_source_device: Option<String>,
    pub(crate) image_capture_method: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) fetched_title: Option<String>,
    pub(crate) thumbnail_s3_key: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) duration_seconds: Option<i32>,
    pub(crate) archive_status: String,
    pub(crate) watch_status: String,
    pub(crate) inbox_status: String,
    pub(crate) notes: String,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) update_cursor: OffsetDateTime,
}

impl ItemRow {
    pub(crate) fn into_detail(
        self,
        tags: Vec<ItemTag>,
        archive_status: ArchiveStatus,
        watch_status: WatchStatus,
        inbox_status: InboxStatus,
    ) -> LibraryItemDetail {
        let item_kind = ItemKind::try_from(self.item_kind.as_str()).unwrap_or(ItemKind::Url);
        let image = image_summary(&self);
        let url = self
            .original_url
            .map(|original_url| ItemUrlSummary::new(original_url, self.canonical_url));
        let text = self
            .plain_text
            .zip(self.content_hash)
            .map(|(plain_text, hash)| {
                ItemTextSummary::new(
                    plain_text,
                    self.html_content,
                    hash,
                    self.text_source_app,
                    self.text_source_device,
                    self.text_capture_method
                        .unwrap_or_else(|| "desktop_clipboard".to_string()),
                )
            });
        LibraryItemDetail {
            summary: LibraryItemSummary {
                id: self.id,
                item_kind,
                url,
                text,
                image,
                title: self.title,
                fetched_title: self.fetched_title,
                thumbnail_s3_key: self.thumbnail_s3_key,
                author: self.author,
                platform: self.platform,
                duration_seconds: self.duration_seconds,
                archive_status,
                watch_status,
                inbox_status,
                tags,
                created_at: self.created_at,
            },
            notes: self.notes,
        }
    }
}

fn image_summary(row: &ItemRow) -> Option<ItemImageSummary> {
    let s3_key = row.image_s3_key.clone()?;
    let content_type = row.image_content_type.clone()?;
    Some(ItemImageSummary {
        s3_key,
        content_type,
        original_filename: row.image_original_filename.clone(),
        byte_size: row.image_byte_size,
        upload_status: upload_status(row.image_upload_status.as_deref()),
        source_app: row.image_source_app.clone(),
        source_device: row.image_source_device.clone(),
        capture_method: row
            .image_capture_method
            .clone()
            .unwrap_or_else(|| "android_share".to_string()),
    })
}

fn upload_status(value: Option<&str>) -> ImageUploadStatus {
    value
        .and_then(|value| ImageUploadStatus::try_from(value).ok())
        .unwrap_or(ImageUploadStatus::Pending)
}

#[derive(sqlx::FromRow)]
pub(crate) struct ItemTagRow {
    id: Uuid,
    display_name: String,
    normalized_name: String,
}

impl From<ItemTagRow> for ItemTag {
    fn from(value: ItemTagRow) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            normalized_name: value.normalized_name,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct TagCorpusRow {
    id: Uuid,
    display_name: String,
    normalized_name: String,
    usage_count: i32,
}

impl From<TagCorpusRow> for TagCorpusEntry {
    fn from(value: TagCorpusRow) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            normalized_name: value.normalized_name,
            usage_count: value.usage_count,
        }
    }
}
