-- `draft_only` items (scripts/flows/apps that were saved as a draft but
-- never deployed) used to live as a stub row in their own table PLUS a
-- `draft` row. Now that drafts are first-class and live in `draft`
-- exclusively, the stub is redundant.
--
-- For every draft_only stub:
--   1. Make sure a workspace-level draft row exists at its path. A real
--      per-user draft already exists for the vast majority of stubs
--      (the autosaver writes one alongside the stub when the user first
--      saves a brand-new item). ON CONFLICT DO NOTHING preserves those
--      and only synthesises a stand-in for the rare stub that lost its
--      draft — same shape the editor's autosave would write. The
--      synthesised row has `email = NULL` (no known owner — a legacy
--      workspace draft shared across the workspace).
--   2. Drop the stub. FKs from script_version / flow_version /
--      app_version cascade.
--   3. Drop the now-unused `draft_only` column.
--
-- ON CONFLICT targets the `draft_pkey_legacy` partial unique index —
-- (workspace_id, path, typ) WHERE email IS NULL.

INSERT INTO draft (workspace_id, path, typ, email, value)
SELECT
    s.workspace_id,
    s.path,
    'script'::DRAFT_KIND,
    NULL,
    jsonb_strip_nulls(jsonb_build_object(
        'path', s.path,
        'summary', s.summary,
        'description', s.description,
        'content', s.content,
        'language', s.language,
        'kind', s.kind,
        'tag', s.tag,
        'schema', s.schema,
        'envs', to_jsonb(s.envs),
        'concurrent_limit', s.concurrent_limit,
        'concurrency_time_window_s', s.concurrency_time_window_s,
        'concurrency_key', s.concurrency_key,
        'cache_ttl', s.cache_ttl,
        'cache_ignore_s3_path', s.cache_ignore_s3_path,
        'dedicated_worker', s.dedicated_worker,
        'ws_error_handler_muted', s.ws_error_handler_muted,
        'priority', s.priority,
        'timeout', s.timeout,
        'delete_after_use', s.delete_after_use,
        'restart_unless_cancelled', s.restart_unless_cancelled,
        'visible_to_runner_only', s.visible_to_runner_only,
        'auto_kind', s.auto_kind,
        'has_preprocessor', s.has_preprocessor,
        'on_behalf_of_email', s.on_behalf_of_email,
        'assets', s.assets,
        'debounce_key', s.debounce_key,
        'debounce_delay_s', s.debounce_delay_s,
        'labels', to_jsonb(s.labels),
        'draft_triggers', '[]'::jsonb
    ))::json
FROM script s
WHERE s.draft_only IS TRUE AND s.deleted IS FALSE AND s.archived IS FALSE
ON CONFLICT (workspace_id, path, typ) WHERE email IS NULL DO NOTHING;

INSERT INTO draft (workspace_id, path, typ, email, value)
SELECT
    f.workspace_id,
    f.path,
    'flow'::DRAFT_KIND,
    NULL,
    jsonb_strip_nulls(jsonb_build_object(
        'path', f.path,
        'summary', f.summary,
        'description', f.description,
        'value', f.value,
        'schema', f.schema,
        'tag', f.tag,
        'dedicated_worker', f.dedicated_worker,
        'timeout', f.timeout,
        'visible_to_runner_only', f.visible_to_runner_only,
        'on_behalf_of_email', f.on_behalf_of_email,
        'ws_error_handler_muted', f.ws_error_handler_muted,
        'labels', to_jsonb(f.labels),
        'draft_triggers', '[]'::jsonb
    ))::json
FROM flow f
WHERE f.draft_only IS TRUE AND f.archived IS FALSE
ON CONFLICT (workspace_id, path, typ) WHERE email IS NULL DO NOTHING;

-- App drafts (raw or not) store the editor's working value directly in
-- `draft.value` — an `App` object for `typ='app'`, a `{files, runnables,
-- data}` object for `typ='raw_app'`. The deployed wrapper's
-- summary/policy/custom_path aren't in the App type and aren't restored
-- from a draft on reload, so we don't bother carrying them through here
-- — the editor renders the stub with default summary/policy on load.
INSERT INTO draft (workspace_id, path, typ, email, value)
SELECT
    a.workspace_id,
    a.path,
    CASE WHEN av.raw_app THEN 'raw_app'::DRAFT_KIND ELSE 'app'::DRAFT_KIND END,
    NULL,
    av.value
FROM app a
JOIN app_version av ON av.id = a.versions[array_upper(a.versions, 1)]
WHERE a.draft_only IS TRUE
ON CONFLICT (workspace_id, path, typ) WHERE email IS NULL DO NOTHING;

DELETE FROM script WHERE draft_only IS TRUE;
DELETE FROM flow WHERE draft_only IS TRUE;
DELETE FROM app WHERE draft_only IS TRUE;

ALTER TABLE script DROP COLUMN draft_only;
ALTER TABLE flow DROP COLUMN draft_only;
ALTER TABLE app DROP COLUMN draft_only;
