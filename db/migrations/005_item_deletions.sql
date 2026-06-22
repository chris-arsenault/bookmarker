CREATE TABLE item_deletions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id    UUID NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT item_deletions_user_item_unique UNIQUE (user_id, item_id)
);

CREATE INDEX item_deletions_user_deleted_at_idx
    ON item_deletions (user_id, deleted_at);
