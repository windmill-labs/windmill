-- Where the profile put this script's relations when the graph was ingested:
-- `<schema>|<database>`. A `profile.resource` is re-read on every run, so a
-- schema or catalog changed on it relocates everything the project builds while
-- the stored rows still name the old relations. Comparing against the deploy
-- lock is not enough — moving the resource A -> B and back to A matches the lock
-- again while these rows are still at B — so the graph records its own root.
ALTER TABLE dbt_node ADD COLUMN IF NOT EXISTS relation_root TEXT;
