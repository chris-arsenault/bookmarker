# Database Migrations

Platform PostgreSQL migrations for the `linkdrop` database.

Migration files follow the Ahara layout:

```text
db/migrations/001_create_tables.sql
db/migrations/rollback/001_create_tables.sql
db/migrations/seed/001_initial_data.sql
```

The migration set defines the durable model for captured URLs, text snippets,
images, original and canonical URLs, explicit tags, tag usage counts, notes,
metadata snapshots, processing jobs, upload status, update polling, and archive
status. Rollback files must drop only project-owned objects in reverse
dependency order.

The Rust integration tests under `backend/shared/tests/` apply this migration
against PostgreSQL, verify constraints and idempotent status updates, and apply
the rollback.

Do not create database roles, users, grants, default privileges, or databases in
project migrations. The Ahara migration service owns those platform concerns.
