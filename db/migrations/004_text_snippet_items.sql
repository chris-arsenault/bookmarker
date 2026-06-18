ALTER TABLE items
ADD COLUMN item_kind TEXT NOT NULL DEFAULT 'url';

ALTER TABLE items
ADD CONSTRAINT items_item_kind_valid
CHECK (item_kind IN ('url', 'text_snippet'));

CREATE TABLE item_texts (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id        UUID NOT NULL,
    user_id        UUID NOT NULL,
    plain_text     TEXT NOT NULL,
    html_content   TEXT,
    content_hash   TEXT NOT NULL,
    source_app     TEXT,
    source_device  TEXT,
    capture_method TEXT NOT NULL DEFAULT 'desktop_clipboard',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT item_texts_item_unique UNIQUE (item_id),
    CONSTRAINT item_texts_item_user_fk
        FOREIGN KEY (item_id, user_id) REFERENCES items(id, user_id) ON DELETE CASCADE,
    CONSTRAINT item_texts_user_fk FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT item_texts_plain_text_not_empty CHECK (btrim(plain_text) <> ''),
    CONSTRAINT item_texts_content_hash_not_empty CHECK (btrim(content_hash) <> ''),
    CONSTRAINT item_texts_capture_method_not_empty CHECK (btrim(capture_method) <> ''),
    CONSTRAINT item_texts_html_content_not_empty
        CHECK (html_content IS NULL OR btrim(html_content) <> ''),
    CONSTRAINT item_texts_source_app_not_empty
        CHECK (source_app IS NULL OR btrim(source_app) <> ''),
    CONSTRAINT item_texts_source_device_not_empty
        CHECK (source_device IS NULL OR btrim(source_device) <> '')
);

CREATE UNIQUE INDEX item_texts_user_content_hash_unique
    ON item_texts (user_id, content_hash);
