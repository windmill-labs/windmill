-- `draft_only` items (scripts/flows/apps that were saved as a draft but never
-- deployed) used to live as a stub row in their own table plus a `draft` row.
-- Now that drafts are first-class, never-deployed items live ONLY in the
-- `draft` table. This migration:
--   1. Makes sure every draft_only stub has a matching `draft` row. A draft row
--      already exists for virtually every draft_only item (it is written
--      alongside the stub when the user saves a brand-new item). ON CONFLICT DO
--      NOTHING preserves those real drafts and only synthesises one for the rare
--      stub that lost its draft. The synthesised value mirrors the JSON the
--      frontend stores for a draft of that kind ({...item fields, draft_triggers}).
--      Transferred rows have email = NULL (no known owner).
--   2. Deletes the stub rows (cascading to *_version / dependency rows).
--   3. Drops the now-unused `draft_only` column.

INSERT INTO draft (workspace_id, path, typ, email, value)
SELECT
    s.workspace_id,
    s.path,
    'script'::DRAFT_TYPE,
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
ON CONFLICT (workspace_id, path, typ, COALESCE(email, '')) DO NOTHING;

INSERT INTO draft (workspace_id, path, typ, email, value)
SELECT
    f.workspace_id,
    f.path,
    'flow'::DRAFT_TYPE,
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
ON CONFLICT (workspace_id, path, typ, COALESCE(email, '')) DO NOTHING;

INSERT INTO draft (workspace_id, path, typ, email, value)
SELECT
    a.workspace_id,
    a.path,
    'app'::DRAFT_TYPE,
    NULL,
    jsonb_build_object(
        'value', av.value,
        'path', a.path,
        'summary', a.summary,
        'policy', a.policy,
        'custom_path', a.custom_path
    )::json
FROM app a
JOIN app_version av ON av.id = a.versions[array_upper(a.versions, 1)]
WHERE a.draft_only IS TRUE
ON CONFLICT (workspace_id, path, typ, COALESCE(email, '')) DO NOTHING;

-- Remove the stub rows now that their content lives in `draft`. FKs from
-- *_version / dependency tables cascade.
DELETE FROM script WHERE draft_only IS TRUE;
DELETE FROM flow WHERE draft_only IS TRUE;
DELETE FROM app WHERE draft_only IS TRUE;

ALTER TABLE script DROP COLUMN draft_only;
ALTER TABLE flow DROP COLUMN draft_only;
ALTER TABLE app DROP COLUMN draft_only;
