-- Repair s3object asset paths recorded without their default-storage leading
-- slash. An S3 asset path is `<storage>/<key>` with an empty storage segment
-- (leading slash) for the workspace default: `s3:///exports/x` -> `/exports/x`.
-- Between 2026-07-06 (#9939) and the parser fix, the asset parser stripped
-- leading slashes, so default-storage assets were recorded as `exports/x` —
-- indistinguishable from a secondary storage named `exports`. For rows created
-- in that window (cutoff one day early for safety), a slashless path whose
-- first segment is NOT a storage name — neither a configured secondary storage
-- nor the reserved `_default_` alias — can only be a default-storage key, so it
-- gets its slash back. Rows already starting with `/` are always correct.
--
-- Best-effort by nature: the corruption itself conflated a stripped default key
-- with a named ref, so identity is inferred from the storage config AS IT IS NOW.
-- A named ref to a storage that was since removed/renamed (or never configured)
-- is the one residual false-positive — it would be repaired as if default. The
-- `created_at` window bounds this for `asset`; `script_trigger` has no timestamp
-- and relies on the storage-name heuristic alone. Both are acceptable given how
-- rare mid-window storage churn is versus the common default-key case this fixes.
--
-- Same corruption hit `script_trigger.trigger_ref` (the pipeline cascade edges,
-- stored as `s3://<path>`): a default-storage edge recorded as `s3://exports/x`
-- instead of `s3:///exports/x` no longer matches the producer's post-fix write
-- ref at dispatch (asset_dispatch rebuilds `s3://` + the repaired asset path and
-- does an exact `trigger_ref =` match), silently breaking the edge. Repaired with
-- the same storage-name heuristic — script_trigger has no created_at, but a
-- correct default ref is always `s3:///…` and a correct named ref always leads
-- with a real storage name, so a `s3://<seg>/…` ref whose seg isn't a storage is
-- unambiguously a slash-stripped default-storage ref.
--
-- `join_pending_inputs.trigger_ref` (the AND-join barrier) is deliberately NOT
-- repaired: it is transient slot state cleared on fire, so a window-era `s3://…`
-- slot is superseded once inputs re-arrive under the corrected ref (and deleting
-- live slots could drop an in-flight accumulation). materialized_asset_schema is
-- unaffected — it only ever holds ducklake asset_kind, never s3object.
--
-- Data-repair only: wrapped so a failure NOTICEs and never blocks the release.
DO $migration$
BEGIN
    CREATE TEMP TABLE __asset_slash_fix_cache (
        workspace_id TEXT PRIMARY KEY,
        names TEXT[] NOT NULL
    ) ON COMMIT DROP;

    -- Reserved first-path-segments that denote a real storage (so a slashless
    -- path leading with one is a genuine named ref, NOT a slash-stripped default
    -- key): the workspace's secondary_storage names PLUS `_default_`, the alias
    -- the runtime treats as the primary storage (workspaces.rs fork_storage_ref).
    -- `s3://_default_/key` is a valid explicit-default ref recorded verbatim as
    -- `_default_/key`; prepending a slash would corrupt it to key `_default_/key`.
    -- The JSON is parsed at most once per workspace (candidate assets can repeat
    -- a workspace millions of times via job usages), and only workspaces that
    -- actually have candidate rows are ever fetched.
    CREATE FUNCTION pg_temp.__asset_slash_fix_storages(ws TEXT) RETURNS TEXT[] AS $fn$
    DECLARE
        result TEXT[];
    BEGIN
        SELECT c.names INTO result FROM __asset_slash_fix_cache c WHERE c.workspace_id = ws;
        IF FOUND THEN
            RETURN result;
        END IF;
        SELECT ARRAY['_default_'] || COALESCE(array_agg(k), '{}') INTO result
        FROM workspace_settings s
        CROSS JOIN LATERAL jsonb_object_keys(
            CASE WHEN jsonb_typeof(s.large_file_storage -> 'secondary_storage') = 'object'
                 THEN s.large_file_storage -> 'secondary_storage'
                 ELSE '{}'::JSONB END
        ) k
        WHERE s.workspace_id = ws;
        result := COALESCE(result, ARRAY['_default_']);
        INSERT INTO __asset_slash_fix_cache VALUES (ws, result);
        RETURN result;
    END
    $fn$ LANGUAGE plpgsql;

    -- Duplicates first: when the corrected `/path` row already exists for the
    -- same usage (recorded before the regression, or re-recorded after the
    -- parser fix), prepending the slash would violate the primary key
    -- (workspace_id, path, kind, usage_path, usage_kind) — drop the slashless
    -- duplicate instead.
    DELETE FROM asset a
    WHERE a.kind = 's3object'
      AND a.created_at > '2026-07-05 00:00:00+00'::TIMESTAMPTZ
      AND a.path NOT LIKE '/%'
      AND a.path <> ''
      AND length(a.path) < 255
      AND split_part(a.path, '/', 1) <> ALL (pg_temp.__asset_slash_fix_storages(a.workspace_id))
      AND EXISTS (
          SELECT 1 FROM asset b
          WHERE b.workspace_id = a.workspace_id
            AND b.path = '/' || a.path
            AND b.kind = a.kind
            AND b.usage_path = a.usage_path
            AND b.usage_kind = a.usage_kind
      );

    -- length < 255 keeps the prepend within the VARCHAR(255) column; a 255-char
    -- slashless path cannot be repaired and is left as-is rather than erroring.
    UPDATE asset a
    SET path = '/' || a.path
    WHERE a.kind = 's3object'
      AND a.created_at > '2026-07-05 00:00:00+00'::TIMESTAMPTZ
      AND a.path NOT LIKE '/%'
      AND a.path <> ''
      AND length(a.path) < 255
      AND split_part(a.path, '/', 1) <> ALL (pg_temp.__asset_slash_fix_storages(a.workspace_id));

    -- script_trigger.trigger_ref for asset edges is `s3://<path>`. A corrupted
    -- default-storage edge reads `s3://<seg>/…` (exactly two slashes); a correct
    -- default ref is `s3:///…` and is excluded by the NOT LIKE. `substring(from 6)`
    -- is the `<path>` after the `s3://` prefix. Delete a slashless edge whose
    -- corrected twin already exists for the same runnable (fetch_subscribers has
    -- no DISTINCT, so a duplicate would double-dispatch the subscriber).
    DELETE FROM script_trigger a
    WHERE a.trigger_kind = 'asset'
      AND a.trigger_ref LIKE 's3://%'
      AND a.trigger_ref NOT LIKE 's3:///%'
      AND substring(a.trigger_ref FROM 6) <> ''
      AND split_part(substring(a.trigger_ref FROM 6), '/', 1)
          <> ALL (pg_temp.__asset_slash_fix_storages(a.workspace_id))
      AND EXISTS (
          SELECT 1 FROM script_trigger b
          WHERE b.workspace_id = a.workspace_id
            AND b.runnable_kind = a.runnable_kind
            AND b.runnable_path = a.runnable_path
            AND b.trigger_kind = a.trigger_kind
            AND b.trigger_ref = 's3:///' || substring(a.trigger_ref FROM 6)
      );

    UPDATE script_trigger a
    SET trigger_ref = 's3:///' || substring(a.trigger_ref FROM 6)
    WHERE a.trigger_kind = 'asset'
      AND a.trigger_ref LIKE 's3://%'
      AND a.trigger_ref NOT LIKE 's3:///%'
      AND substring(a.trigger_ref FROM 6) <> ''
      AND split_part(substring(a.trigger_ref FROM 6), '/', 1)
          <> ALL (pg_temp.__asset_slash_fix_storages(a.workspace_id));

    -- The temp table is ON COMMIT DROP; drop the function too so nothing
    -- lingers on a pooled connection.
    DROP FUNCTION pg_temp.__asset_slash_fix_storages(TEXT);
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'skipping s3 asset leading-slash repair: %', SQLERRM;
END
$migration$;
