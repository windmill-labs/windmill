-- Repair s3object asset paths recorded without their default-storage leading
-- slash. An S3 asset path is `<storage>/<key>` with an empty storage segment
-- (leading slash) for the workspace default: `s3:///exports/x` -> `/exports/x`.
-- Between 2026-07-06 (#9939) and the parser fix, the asset parser stripped
-- leading slashes, so default-storage assets were recorded as `exports/x` —
-- indistinguishable from a secondary storage named `exports`. For rows created
-- in that window (cutoff one day early for safety), a slashless path whose
-- first segment is NOT one of the workspace's secondary storage names can only
-- be a default-storage key, so it gets its slash back. Rows already starting
-- with `/` are always correct.
--
-- Data-repair only: wrapped so a failure NOTICEs and never blocks the release.
DO $migration$
BEGIN
    CREATE TEMP TABLE __asset_slash_fix_cache (
        workspace_id TEXT PRIMARY KEY,
        names TEXT[] NOT NULL
    ) ON COMMIT DROP;

    -- Secondary storage names of a workspace, from the keys of
    -- workspace_settings.large_file_storage->'secondary_storage'. The JSON is
    -- parsed at most once per workspace (candidate assets can repeat a
    -- workspace millions of times via job usages), and only workspaces that
    -- actually have candidate rows are ever fetched.
    CREATE FUNCTION pg_temp.__asset_slash_fix_storages(ws TEXT) RETURNS TEXT[] AS $fn$
    DECLARE
        result TEXT[];
    BEGIN
        SELECT c.names INTO result FROM __asset_slash_fix_cache c WHERE c.workspace_id = ws;
        IF FOUND THEN
            RETURN result;
        END IF;
        SELECT COALESCE(array_agg(k), '{}') INTO result
        FROM workspace_settings s
        CROSS JOIN LATERAL jsonb_object_keys(
            CASE WHEN jsonb_typeof(s.large_file_storage -> 'secondary_storage') = 'object'
                 THEN s.large_file_storage -> 'secondary_storage'
                 ELSE '{}'::JSONB END
        ) k
        WHERE s.workspace_id = ws;
        result := COALESCE(result, '{}');
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

    -- The temp table is ON COMMIT DROP; drop the function too so nothing
    -- lingers on a pooled connection.
    DROP FUNCTION pg_temp.__asset_slash_fix_storages(TEXT);
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'skipping s3 asset leading-slash repair: %', SQLERRM;
END
$migration$;
