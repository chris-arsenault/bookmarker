ALTER TABLE items
DROP CONSTRAINT IF EXISTS items_inbox_status_valid;

ALTER TABLE items
DROP COLUMN IF EXISTS inbox_status;
