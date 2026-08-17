-- `draft_only` items (scripts/flows/apps saved as a draft but never
-- deployed) lived as a stub row in their own table plus a `draft` row; the
-- stub is now redundant. For each stub: ensure a draft row exists at its
-- path (ON CONFLICT DO NOTHING preserves the real per-user draft most stubs
-- already have, synthesising an `email = NULL` legacy stand-in only for the
-- rare stub that lost its draft), drop the stub (version FKs cascade), then
-- drop the `draft_only` column.
-- ON CONFLICT targets `draft_pkey_legacy` — (workspace_id, path, typ) WHERE
-- email IS NULL.

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

-- App drafts store the editor's working value directly in `draft.value` (an
-- `App` object for `app`, a `{files, runnables, data}` object for `raw_app`).
-- The deployed wrapper's summary/policy/custom_path aren't part of the App
-- type and aren't restored from a draft on reload, so they're dropped here.
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
