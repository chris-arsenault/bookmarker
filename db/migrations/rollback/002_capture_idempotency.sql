DROP INDEX IF EXISTS items_user_client_capture_id_unique;

ALTER TABLE items
DROP CONSTRAINT IF EXISTS items_client_capture_id_not_empty;

ALTER TABLE items
DROP COLUMN IF EXISTS client_capture_id;
