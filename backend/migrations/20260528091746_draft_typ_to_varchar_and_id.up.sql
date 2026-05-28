-- Add up migration script here
-- Loosen draft.typ from the DRAFT_TYPE enum to a plain VARCHAR so the
-- bidirectional sync layer can store drafts for any UserDraft item kind
-- (raw_app, resource, variable, trigger_*, ...) without an enum migration
-- per kind.
ALTER TABLE draft ALTER COLUMN typ TYPE VARCHAR(50) USING typ::text;
DROP TYPE DRAFT_TYPE;

-- Give the draft table a proper synthetic primary key. The previous
-- migration replaced the composite PK with two partial unique indexes
-- (one for user-scoped rows, one for legacy NULL-username rows); having
-- no PK at all breaks tools that assume one (pg_dump, replication, ORM
-- drift detection).
ALTER TABLE draft ADD COLUMN id BIGSERIAL PRIMARY KEY;
