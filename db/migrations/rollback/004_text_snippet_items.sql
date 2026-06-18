DROP INDEX IF EXISTS item_texts_user_content_hash_unique;

DROP TABLE IF EXISTS item_texts;

ALTER TABLE items
DROP CONSTRAINT IF EXISTS items_item_kind_valid;

ALTER TABLE items
DROP COLUMN IF EXISTS item_kind;
