ALTER TABLE items
ADD COLUMN title TEXT;

ALTER TABLE items
ADD CONSTRAINT items_title_not_empty
CHECK (title IS NULL OR btrim(title) <> '');
