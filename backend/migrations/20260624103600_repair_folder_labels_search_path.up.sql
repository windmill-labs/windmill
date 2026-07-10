-- Repair instances that already applied the folder-labels migrations while the
-- function hardcoded `SET search_path = public`. On a non-public schema (PG_SCHEMA)
-- the function was pinned to `public`, so at runtime it read the wrong `folder`
-- table (or a stray public.folder) instead of the workspace's real one.
--
-- `FROM CURRENT` snapshots the migration connection's search_path (the actual
-- Windmill schema) into the function, keeping the SECURITY DEFINER injection
-- hardening. On public-schema installs this re-pins to `public`, i.e. a no-op.
-- Idempotent: redefining with the same body is harmless on already-correct installs.
CREATE OR REPLACE FUNCTION folder_labels(w_id text, item_path text) RETURNS text[]
LANGUAGE sql STABLE SECURITY DEFINER SET search_path FROM CURRENT AS $$
    SELECT (
        SELECT array_agg(l ORDER BY first_ord)
        FROM (
            SELECT u.l, min(u.ord) AS first_ord
            FROM unnest(f.labels) WITH ORDINALITY AS u(l, ord)
            GROUP BY u.l
        ) deduped
    )
    FROM folder f
    WHERE f.workspace_id = w_id AND item_path LIKE 'f/%' AND f.name = split_part(item_path, '/', 2)
$$;
