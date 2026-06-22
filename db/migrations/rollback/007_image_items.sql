DROP TABLE IF EXISTS item_images;

ALTER TABLE items
DROP CONSTRAINT IF EXISTS items_item_kind_valid;

ALTER TABLE items
ADD CONSTRAINT items_item_kind_valid
CHECK (item_kind IN ('url', 'text_snippet'));
