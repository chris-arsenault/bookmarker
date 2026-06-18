ALTER TABLE items
ADD COLUMN client_capture_id TEXT;

ALTER TABLE items
ADD CONSTRAINT items_client_capture_id_not_empty
CHECK (client_capture_id IS NULL OR btrim(client_capture_id) <> '');

CREATE UNIQUE INDEX items_user_client_capture_id_unique
ON items (user_id, client_capture_id)
WHERE client_capture_id IS NOT NULL;
