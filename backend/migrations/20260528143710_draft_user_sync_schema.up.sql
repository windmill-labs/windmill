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

ALTER TABLE draft ADD COLUMN id BIGSERIAL PRIMARY KEY;
