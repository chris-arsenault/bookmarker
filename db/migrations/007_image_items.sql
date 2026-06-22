ALTER TABLE items
DROP CONSTRAINT IF EXISTS items_item_kind_valid;

ALTER TABLE items
ADD CONSTRAINT items_item_kind_valid
CHECK (item_kind IN ('url', 'text_snippet', 'image'));

CREATE TABLE item_images (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id           UUID NOT NULL,
    user_id           UUID NOT NULL,
    s3_key            TEXT NOT NULL,
    content_type      TEXT NOT NULL,
    original_filename TEXT,
    byte_size         BIGINT,
    upload_status     TEXT NOT NULL DEFAULT 'pending',
    source_app        TEXT,
    source_device     TEXT,
    capture_method    TEXT NOT NULL DEFAULT 'android_share',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT item_images_item_unique UNIQUE (item_id),
    CONSTRAINT item_images_user_s3_key_unique UNIQUE (user_id, s3_key),
    CONSTRAINT item_images_item_user_fk
        FOREIGN KEY (item_id, user_id) REFERENCES items(id, user_id) ON DELETE CASCADE,
    CONSTRAINT item_images_user_fk FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT item_images_s3_key_not_empty CHECK (btrim(s3_key) <> ''),
    CONSTRAINT item_images_content_type_image
        CHECK (content_type LIKE 'image/%' AND btrim(content_type) <> ''),
    CONSTRAINT item_images_original_filename_not_empty
        CHECK (original_filename IS NULL OR btrim(original_filename) <> ''),
    CONSTRAINT item_images_byte_size_positive
        CHECK (byte_size IS NULL OR byte_size > 0),
    CONSTRAINT item_images_upload_status_valid
        CHECK (upload_status IN ('pending', 'uploaded', 'failed')),
    CONSTRAINT item_images_source_app_not_empty
        CHECK (source_app IS NULL OR btrim(source_app) <> ''),
    CONSTRAINT item_images_source_device_not_empty
        CHECK (source_device IS NULL OR btrim(source_device) <> ''),
    CONSTRAINT item_images_capture_method_not_empty CHECK (btrim(capture_method) <> '')
);
