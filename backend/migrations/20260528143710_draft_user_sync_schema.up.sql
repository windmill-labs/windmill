-- Reshape `draft` for per-user bidirectional sync: add the owner `email`
-- (FK to password.email, NULL on legacy rows); replace the composite PK with
-- two partial unique indexes so per-user rows and the single legacy
-- workspace-level row coexist at the same (workspace_id, path, typ); widen
-- the DRAFT_TYPE enum to DRAFT_KIND (every UserDraftItemKind); and add a
-- synthetic BIGSERIAL `id` PK (tools like pg_dump/replication break on the
-- partial-index-only layout).

CREATE TYPE DRAFT_KIND AS ENUM (
    'script',
    'flow',
    'app',
    'raw_app',
    'resource',
    'variable',
    'trigger_schedule',
    'trigger_webhook',
    'trigger_default_email',
    'trigger_email',
    'trigger_http',
    'trigger_websocket',
    'trigger_postgres',
    'trigger_kafka',
    'trigger_nats',
    'trigger_mqtt',
    'trigger_sqs',
    'trigger_gcp',
    'trigger_azure',
    'trigger_poll',
    'trigger_cli',
    'trigger_nextcloud',
    'trigger_google',
    'trigger_github'
);

ALTER TABLE draft ALTER COLUMN typ TYPE DRAFT_KIND USING typ::text::DRAFT_KIND;
DROP TYPE DRAFT_TYPE;

ALTER TABLE draft ADD COLUMN email VARCHAR(255);

ALTER TABLE draft
    ADD CONSTRAINT draft_password_fkey
    FOREIGN KEY (email)
    REFERENCES password(email)
    ON DELETE CASCADE
    ON UPDATE CASCADE;

ALTER TABLE draft DROP CONSTRAINT draft_pkey;

CREATE UNIQUE INDEX draft_pkey_with_user
    ON draft (workspace_id, path, typ, email)
    WHERE email IS NOT NULL;

CREATE UNIQUE INDEX draft_pkey_legacy
    ON draft (workspace_id, path, typ)
    WHERE email IS NULL;

-- Serves the per-user draft listing (`WHERE workspace_id = ? AND email = ?
-- ORDER BY path`); neither partial unique index helps (both lead with
-- `path, typ`), and the trailing `path` keeps rows in output order.
CREATE INDEX draft_user_listing_idx
    ON draft (workspace_id, email, path)
    WHERE email IS NOT NULL;

ALTER TABLE draft ADD COLUMN id BIGSERIAL PRIMARY KEY;

-- Hot path: `fetch_other_drafts_users` runs on every get-by-path request
-- with `WHERE workspace_id = ? AND path = ? AND typ = ?` and no email
-- predicate. The partial unique/listing indexes can't serve it (their
-- `email IS [NOT] NULL` predicates aren't implied by the query), so a plain
-- btree is needed. Also covers `get_draft_for_user` (same three columns +
-- `email IS NOT DISTINCT FROM ?` as a filter).
CREATE INDEX draft_workspace_path_typ_idx ON draft (workspace_id, path, typ);

-- Secret variable values must never sit in `draft.value` in plaintext.
-- `save_draft` now encrypts them at write time; this scrubs any rows
-- persisted before that guard (irreversible — see the .down.sql note).
UPDATE draft
SET value = jsonb_set(value::jsonb, '{variable,value}', '""'::jsonb)::json
WHERE typ = 'variable'
  AND (value::jsonb -> 'variable' ->> 'is_secret')::boolean IS TRUE
  AND value::jsonb -> 'variable' ? 'value';
