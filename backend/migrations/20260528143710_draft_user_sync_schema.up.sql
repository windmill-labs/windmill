-- Reshape `draft` for per-user bidirectional sync:
--   * add `email` (FK to `password.email`) — owner of the draft. NULL on
--     legacy rows written before per-user sync existed.
--   * replace the composite PK with two partial unique indexes so per-user
--     rows and the single legacy workspace-level row can coexist at the
--     same (workspace_id, path, typ).
--   * replace the DRAFT_TYPE enum (script/flow/app only) with DRAFT_KIND,
--     covering every UserDraftItemKind the sync layer accepts. Keeping it
--     as an enum (rather than VARCHAR) lets the type system reject typos
--     at the DB boundary and stays in sync with the Rust `UserDraftItemKind`.
--   * give the table a synthetic BIGSERIAL `id` PK — tools that assume a
--     real PK (pg_dump, replication, ORM drift detection) break on the
--     partial-index-only layout above.

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

-- Serves the per-user draft listing (`GET /drafts/list`:
-- `WHERE workspace_id = ? AND email = ? ORDER BY path`). Neither partial
-- unique index above helps — both lead with `path, typ` — and the
-- trailing `path` lets the planner read rows already in output order.
CREATE INDEX draft_user_listing_idx
    ON draft (workspace_id, email, path)
    WHERE email IS NOT NULL;

ALTER TABLE draft ADD COLUMN id BIGSERIAL PRIMARY KEY;

-- Hot path: `fetch_other_drafts_users` runs on EVERY get-by-path request
-- (scripts, flows, apps, variables, resources, schedules, triggers) with
-- `WHERE workspace_id = ? AND path = ? AND typ = ?` and no email
-- predicate. Neither partial unique index (`draft_pkey_with_user` /
-- `draft_pkey_legacy`) can serve it — their `email IS [NOT] NULL`
-- predicates aren't implied by the query — and `draft_user_sync_idx` is
-- partial too, so the planner fell back to a sequential scan over a
-- table that accumulates per-user autosaves across all workspaces.
-- A plain btree over the three columns also covers `get_draft_for_user`
-- (same three + `email IS NOT DISTINCT FROM ?` as a filter).
CREATE INDEX draft_workspace_path_typ_idx ON draft (workspace_id, path, typ);

-- Secret variable values must never sit in `draft.value` in plaintext
-- (deployed secrets are encrypted with the workspace crypt key precisely
-- so DB dumps don't leak them). `save_draft` encrypts `variable.value`
-- for `is_secret: true` drafts at write time; this scrubs any rows that
-- were persisted before that guard existed (irreversible — the plaintext
-- is deliberately destroyed, see the .down.sql note).
UPDATE draft
SET value = jsonb_set(value::jsonb, '{variable,value}', '""'::jsonb)::json
WHERE typ = 'variable'
  AND (value::jsonb -> 'variable' ->> 'is_secret')::boolean IS TRUE
  AND value::jsonb -> 'variable' ? 'value';
