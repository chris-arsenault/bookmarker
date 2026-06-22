pub(super) const USER_ID_BY_SUB: &str = "SELECT id FROM users WHERE cognito_sub = $1";
pub(super) const UPSERT_USER: &str = "
    INSERT INTO users (cognito_sub)
    VALUES ($1)
    ON CONFLICT (cognito_sub)
    DO UPDATE SET updated_at = now()
    RETURNING id";
pub(super) const ITEM_SELECT: &str = "
    SELECT
        items.id,
        items.item_kind,
        item_urls.original_url,
        item_urls.canonical_url,
        item_texts.plain_text,
        item_texts.html_content,
        item_texts.content_hash,
        item_texts.source_app AS text_source_app,
        item_texts.source_device AS text_source_device,
        item_texts.capture_method AS text_capture_method,
        item_images.s3_key AS image_s3_key,
        item_images.content_type AS image_content_type,
        item_images.original_filename AS image_original_filename,
        item_images.byte_size AS image_byte_size,
        item_images.upload_status AS image_upload_status,
        item_images.source_app AS image_source_app,
        item_images.source_device AS image_source_device,
        item_images.capture_method AS image_capture_method,
        items.title,
        metadata_snapshots.title AS fetched_title,
        metadata_snapshots.thumbnail_s3_key,
        metadata_snapshots.author,
        metadata_snapshots.platform,
        metadata_snapshots.duration_seconds,
        CASE
            WHEN items.item_kind = 'text_snippet' THEN 'not_applicable'
            WHEN items.item_kind = 'image' THEN
                CASE item_images.upload_status
                    WHEN 'uploaded' THEN 'succeeded'
                    WHEN 'failed' THEN 'failed'
                    ELSE 'pending'
                END
            ELSE COALESCE(metadata_snapshots.archive_status, 'pending')
        END AS archive_status,
        items.watch_status,
        items.inbox_status,
        COALESCE(item_notes.body, '') AS notes,
        items.created_at,
        GREATEST(
            items.updated_at,
            COALESCE(item_urls.updated_at, items.updated_at),
            COALESCE(item_texts.updated_at, items.updated_at),
            COALESCE(item_images.updated_at, items.updated_at),
            COALESCE(metadata_snapshots.updated_at, items.updated_at),
            COALESCE(item_notes.updated_at, items.updated_at)
        ) AS update_cursor
    FROM items
    LEFT JOIN item_urls ON item_urls.item_id = items.id
    LEFT JOIN item_texts ON item_texts.item_id = items.id
    LEFT JOIN item_images ON item_images.item_id = items.id
    LEFT JOIN metadata_snapshots ON metadata_snapshots.item_id = items.id
    LEFT JOIN item_notes ON item_notes.item_id = items.id
    WHERE items.user_id = $1";
pub(super) const LIST_ITEMS: &str = "
      AND ($2::text IS NULL OR lower(metadata_snapshots.platform) = $2)
      AND (
        $3::text IS NULL
        OR EXISTS (
            SELECT 1
            FROM item_tags
            JOIN tags ON tags.id = item_tags.tag_id
            WHERE item_tags.item_id = items.id
              AND (tags.normalized_name = $3 OR lower(tags.display_name) = $3)
        )
      )
      AND ($4::timestamptz IS NULL OR items.created_at >= $4)
      AND ($5::timestamptz IS NULL OR items.created_at <= $5)
      AND ($6::text IS NULL OR
        CASE
            WHEN items.item_kind = 'text_snippet' THEN 'not_applicable'
            WHEN items.item_kind = 'image' THEN
                CASE item_images.upload_status
                    WHEN 'uploaded' THEN 'succeeded'
                    WHEN 'failed' THEN 'failed'
                    ELSE 'pending'
                END
            ELSE COALESCE(metadata_snapshots.archive_status, 'pending')
        END = $6
      )
      AND ($7::text IS NULL OR items.watch_status = $7)
      AND ($8::text IS NULL OR items.inbox_status = $8)
      AND (
        $9::text IS NULL
        OR strpos(lower(COALESCE(items.title, '')), $9) > 0
        OR strpos(lower(COALESCE(metadata_snapshots.title, '')), $9) > 0
        OR strpos(lower(COALESCE(item_notes.body, '')), $9) > 0
        OR strpos(lower(COALESCE(item_texts.plain_text, '')), $9) > 0
        OR strpos(lower(COALESCE(item_images.original_filename, '')), $9) > 0
      )";
pub(super) const LIST_ITEMS_ORDER: &str = "
    ORDER BY items.created_at DESC, items.id";
pub(super) const LIST_ITEM_UPDATES: &str = "
      AND ($2::text IS NULL OR lower(metadata_snapshots.platform) = $2)
      AND (
        $3::text IS NULL
        OR EXISTS (
            SELECT 1
            FROM item_tags
            JOIN tags ON tags.id = item_tags.tag_id
            WHERE item_tags.item_id = items.id
              AND (tags.normalized_name = $3 OR lower(tags.display_name) = $3)
        )
      )
      AND ($4::timestamptz IS NULL OR items.created_at >= $4)
      AND ($5::timestamptz IS NULL OR items.created_at <= $5)
      AND ($6::text IS NULL OR
        CASE
            WHEN items.item_kind = 'text_snippet' THEN 'not_applicable'
            WHEN items.item_kind = 'image' THEN
                CASE item_images.upload_status
                    WHEN 'uploaded' THEN 'succeeded'
                    WHEN 'failed' THEN 'failed'
                    ELSE 'pending'
                END
            ELSE COALESCE(metadata_snapshots.archive_status, 'pending')
        END = $6
      )
      AND ($7::text IS NULL OR items.watch_status = $7)
      AND ($8::text IS NULL OR items.inbox_status = $8)
      AND (
        $9::text IS NULL
        OR strpos(lower(COALESCE(items.title, '')), $9) > 0
        OR strpos(lower(COALESCE(metadata_snapshots.title, '')), $9) > 0
        OR strpos(lower(COALESCE(item_notes.body, '')), $9) > 0
        OR strpos(lower(COALESCE(item_texts.plain_text, '')), $9) > 0
        OR strpos(lower(COALESCE(item_images.original_filename, '')), $9) > 0
      )
      AND GREATEST(
        items.updated_at,
        COALESCE(item_urls.updated_at, items.updated_at),
        COALESCE(item_texts.updated_at, items.updated_at),
        COALESCE(item_images.updated_at, items.updated_at),
        COALESCE(metadata_snapshots.updated_at, items.updated_at),
        COALESCE(item_notes.updated_at, items.updated_at)
      ) > $10
    ORDER BY update_cursor ASC, items.id
    LIMIT $11";
pub(super) const LIST_ITEM_DELETIONS: &str = "
    SELECT item_id, deleted_at
    FROM item_deletions
    WHERE user_id = $1
      AND deleted_at > $2
    ORDER BY deleted_at ASC, item_id
    LIMIT $3";
pub(super) const GET_ITEM: &str = "
      AND items.id = $2";
pub(super) const GET_ITEM_BY_CAPTURE_ID: &str = "
      AND items.client_capture_id = $2";
pub(super) const GET_ITEM_BY_CANONICAL_URL: &str = "
      AND item_urls.canonical_url = $2";
pub(super) const GET_ITEM_BY_TEXT_HASH: &str = "
      AND item_texts.content_hash = $2";
pub(super) const UPDATE_IMAGE_UPLOAD_STATUS: &str = "
    UPDATE item_images
    SET upload_status = $3, updated_at = now()
    WHERE item_id = $1 AND user_id = $2";
pub(super) const ITEM_TAGS: &str = "
    SELECT tags.id, tags.display_name, tags.normalized_name
    FROM item_tags
    JOIN tags ON tags.id = item_tags.tag_id
    WHERE item_tags.item_id = $1
    ORDER BY tags.normalized_name ASC";
pub(super) const TAG_CORPUS: &str = "
    SELECT
        tags.id,
        tags.display_name,
        tags.normalized_name,
        COALESCE(tag_usage_counts.usage_count, 0) AS usage_count
    FROM tags
    LEFT JOIN tag_usage_counts ON tag_usage_counts.tag_id = tags.id
    WHERE tags.user_id = $1
      AND COALESCE(tag_usage_counts.usage_count, 0) > 0
    ORDER BY COALESCE(tag_usage_counts.usage_count, 0) DESC, tags.normalized_name ASC";
pub(super) const UPDATE_ITEM_ORGANIZATION: &str = "
    UPDATE items
    SET
        watch_status = COALESCE($3, watch_status),
        inbox_status = COALESCE($4, inbox_status),
        title = CASE WHEN $5 THEN $6 ELSE title END,
        updated_at = now()
    WHERE id = $1 AND user_id = $2";
pub(super) const UPSERT_ITEM_NOTE: &str = "
    INSERT INTO item_notes (item_id, user_id, body)
    VALUES ($1, $2, $3)
    ON CONFLICT (item_id)
    DO UPDATE SET body = $3, updated_at = now()";
pub(super) const DELETE_ITEM_TAGS: &str = "
    DELETE FROM item_tags
    WHERE item_id = $1 AND user_id = $2";
pub(super) const RECORD_ITEM_DELETION: &str = "
    INSERT INTO item_deletions (user_id, item_id)
    VALUES ($1, $2)
    ON CONFLICT (user_id, item_id)
    DO UPDATE SET deleted_at = now()";
pub(super) const DELETE_ITEM: &str = "
    DELETE FROM items
    WHERE id = $1 AND user_id = $2";
pub(super) const INSERT_CAPTURE_ITEM: &str = "
    INSERT INTO items (user_id, client_capture_id, item_kind, title)
    VALUES ($1, $2, $3, $4)
    RETURNING id";
pub(super) const INSERT_CAPTURE_URL: &str = "
    INSERT INTO item_urls (
        item_id,
        user_id,
        original_url,
        canonical_url,
        normalization_status,
        normalization_error
    )
    VALUES ($1, $2, $3, $4, $5, $6)
    ON CONFLICT (user_id, canonical_url)
    WHERE canonical_url IS NOT NULL
    DO NOTHING";
pub(super) const INSERT_TEXT_CAPTURE: &str = "
    INSERT INTO item_texts (
        item_id,
        user_id,
        plain_text,
        html_content,
        content_hash,
        source_app,
        source_device,
        capture_method
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    ON CONFLICT (user_id, content_hash)
    DO NOTHING";
pub(super) const INSERT_IMAGE_CAPTURE: &str = "
    INSERT INTO item_images (
        item_id,
        user_id,
        s3_key,
        content_type,
        original_filename,
        byte_size,
        source_app,
        source_device,
        capture_method
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
pub(super) const UPSERT_TAG: &str = "
    INSERT INTO tags (user_id, display_name)
    VALUES ($1, $2)
    ON CONFLICT (user_id, normalized_name)
    DO UPDATE SET updated_at = tags.updated_at
    RETURNING id";
pub(super) const INSERT_ITEM_TAG: &str = "
    INSERT INTO item_tags (item_id, tag_id, user_id, applied_source)
    VALUES ($1, $2, $3, 'explicit')
    ON CONFLICT DO NOTHING";
pub(super) const TAG_BY_ID: &str = "
    SELECT id
    FROM tags
    WHERE id = $1 AND user_id = $2";
pub(super) const TAG_RENAME_COLLISION: &str = "
    SELECT id
    FROM tags
    WHERE user_id = $1
      AND normalized_name = $2
      AND id <> $3";
pub(super) const UPDATE_TAG_DISPLAY_NAME: &str = "
    UPDATE tags
    SET display_name = $3, updated_at = now()
    WHERE id = $1 AND user_id = $2";
pub(super) const MERGE_ITEM_TAGS: &str = "
    INSERT INTO item_tags (item_id, tag_id, user_id, applied_source)
    SELECT item_id, $3, user_id, applied_source
    FROM item_tags
    WHERE tag_id = $1 AND user_id = $2
    ON CONFLICT DO NOTHING";
pub(super) const DELETE_SOURCE_ITEM_TAGS: &str = "
    DELETE FROM item_tags
    WHERE tag_id = $1 AND user_id = $2";
pub(super) const DELETE_TAG_BY_ID: &str = "
    DELETE FROM tags
    WHERE id = $1 AND user_id = $2";
