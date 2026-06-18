CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cognito_sub TEXT NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT users_cognito_sub_not_empty CHECK (btrim(cognito_sub) <> '')
);

CREATE TABLE items (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    watch_status TEXT NOT NULL DEFAULT 'unwatched',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT items_id_user_id_unique UNIQUE (id, user_id),
    CONSTRAINT items_watch_status_valid CHECK (watch_status IN ('unwatched', 'watched'))
);

CREATE TABLE item_urls (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id              UUID NOT NULL,
    user_id              UUID NOT NULL,
    original_url         TEXT NOT NULL,
    canonical_url        TEXT,
    normalization_status TEXT NOT NULL DEFAULT 'pending',
    normalization_error  TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT item_urls_item_unique UNIQUE (item_id),
    CONSTRAINT item_urls_item_user_fk
        FOREIGN KEY (item_id, user_id) REFERENCES items(id, user_id) ON DELETE CASCADE,
    CONSTRAINT item_urls_user_fk FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT item_urls_original_url_not_empty CHECK (btrim(original_url) <> ''),
    CONSTRAINT item_urls_canonical_url_not_empty
        CHECK (canonical_url IS NULL OR btrim(canonical_url) <> ''),
    CONSTRAINT item_urls_normalization_status_valid
        CHECK (normalization_status IN ('pending', 'succeeded', 'failed'))
);

CREATE UNIQUE INDEX item_urls_user_canonical_url_unique
    ON item_urls (user_id, canonical_url)
    WHERE canonical_url IS NOT NULL;

CREATE TABLE tags (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    display_name    TEXT NOT NULL,
    normalized_name TEXT GENERATED ALWAYS AS (lower(btrim(display_name))) STORED,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT tags_id_user_id_unique UNIQUE (id, user_id),
    CONSTRAINT tags_user_normalized_name_unique UNIQUE (user_id, normalized_name),
    CONSTRAINT tags_display_name_not_empty CHECK (btrim(display_name) <> '')
);

CREATE TABLE item_tags (
    item_id        UUID NOT NULL,
    tag_id         UUID NOT NULL,
    user_id        UUID NOT NULL,
    applied_source TEXT NOT NULL DEFAULT 'explicit',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT item_tags_pk PRIMARY KEY (item_id, tag_id),
    CONSTRAINT item_tags_item_user_fk
        FOREIGN KEY (item_id, user_id) REFERENCES items(id, user_id) ON DELETE CASCADE,
    CONSTRAINT item_tags_tag_user_fk
        FOREIGN KEY (tag_id, user_id) REFERENCES tags(id, user_id) ON DELETE CASCADE,
    CONSTRAINT item_tags_source_explicit CHECK (applied_source = 'explicit')
);

CREATE TABLE tag_usage_counts (
    tag_id      UUID PRIMARY KEY,
    user_id     UUID NOT NULL,
    usage_count INTEGER NOT NULL DEFAULT 0,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT tag_usage_counts_tag_user_fk
        FOREIGN KEY (tag_id, user_id) REFERENCES tags(id, user_id) ON DELETE CASCADE,
    CONSTRAINT tag_usage_counts_nonnegative CHECK (usage_count >= 0)
);

CREATE FUNCTION update_tag_usage_counts()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO tag_usage_counts (tag_id, user_id, usage_count)
        VALUES (NEW.tag_id, NEW.user_id, 1)
        ON CONFLICT (tag_id)
        DO UPDATE SET
            usage_count = tag_usage_counts.usage_count + 1,
            updated_at = now();
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE tag_usage_counts
        SET usage_count = greatest(usage_count - 1, 0),
            updated_at = now()
        WHERE tag_id = OLD.tag_id;
        RETURN OLD;
    END IF;

    RETURN NULL;
END;
$$;

CREATE TRIGGER item_tags_usage_insert
AFTER INSERT ON item_tags
FOR EACH ROW
EXECUTE FUNCTION update_tag_usage_counts();

CREATE TRIGGER item_tags_usage_delete
AFTER DELETE ON item_tags
FOR EACH ROW
EXECUTE FUNCTION update_tag_usage_counts();

CREATE TABLE item_notes (
    item_id    UUID PRIMARY KEY,
    user_id    UUID NOT NULL,
    body       TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT item_notes_item_user_fk
        FOREIGN KEY (item_id, user_id) REFERENCES items(id, user_id) ON DELETE CASCADE
);

CREATE TABLE metadata_snapshots (
    item_id                UUID PRIMARY KEY,
    user_id                UUID NOT NULL,
    title                  TEXT,
    thumbnail_s3_key       TEXT,
    thumbnail_content_type TEXT,
    author                 TEXT,
    platform               TEXT,
    duration_seconds       INTEGER,
    archive_status         TEXT NOT NULL DEFAULT 'pending',
    archive_error          TEXT,
    captured_at            TIMESTAMPTZ,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT metadata_snapshots_item_user_fk
        FOREIGN KEY (item_id, user_id) REFERENCES items(id, user_id) ON DELETE CASCADE,
    CONSTRAINT metadata_snapshots_duration_nonnegative
        CHECK (duration_seconds IS NULL OR duration_seconds >= 0),
    CONSTRAINT metadata_snapshots_archive_status_valid
        CHECK (archive_status IN ('pending', 'succeeded', 'failed'))
);

CREATE TABLE processing_jobs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id         UUID NOT NULL,
    user_id         UUID NOT NULL,
    job_kind        TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'queued',
    attempt_count   INTEGER NOT NULL DEFAULT 0,
    idempotency_key TEXT NOT NULL,
    available_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_at       TIMESTAMPTZ,
    locked_by       TEXT,
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT processing_jobs_item_user_fk
        FOREIGN KEY (item_id, user_id) REFERENCES items(id, user_id) ON DELETE CASCADE,
    CONSTRAINT processing_jobs_item_kind_unique UNIQUE (item_id, job_kind),
    CONSTRAINT processing_jobs_idempotency_key_unique UNIQUE (idempotency_key),
    CONSTRAINT processing_jobs_job_kind_valid
        CHECK (job_kind IN ('normalize_url', 'enrich_metadata', 'snapshot_thumbnail')),
    CONSTRAINT processing_jobs_status_valid
        CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
    CONSTRAINT processing_jobs_attempt_count_nonnegative CHECK (attempt_count >= 0),
    CONSTRAINT processing_jobs_idempotency_key_not_empty CHECK (btrim(idempotency_key) <> '')
);
