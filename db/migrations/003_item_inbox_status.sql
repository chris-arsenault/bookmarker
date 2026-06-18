ALTER TABLE items
ADD COLUMN inbox_status TEXT NOT NULL DEFAULT 'unsorted';

ALTER TABLE items
ADD CONSTRAINT items_inbox_status_valid
CHECK (inbox_status IN ('unsorted', 'organized'));
