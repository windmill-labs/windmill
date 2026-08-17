-- Folder labels are exposed verbatim as `inherited_labels` (via folder_labels) and
-- rendered in keyed `{#each}` blocks in the UI, which throw `each_key_duplicate` on a
-- repeated key. The write paths now dedup, but make the read resilient regardless of
-- how a row was populated, and clean up any duplicates already persisted.

-- Dedup while preserving first-seen order.
CREATE OR REPLACE FUNCTION folder_labels(w_id text, item_path text) RETURNS text[]
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public AS $$
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

-- One-time cleanup of rows that already contain duplicate labels, so direct reads of
-- folder.labels (folder list, editor) are also safe.
UPDATE folder
SET labels = (
    SELECT array_agg(l ORDER BY first_ord)
    FROM (
        SELECT u.l, min(u.ord) AS first_ord
        FROM unnest(labels) WITH ORDINALITY AS u(l, ord)
        GROUP BY u.l
    ) deduped
)
WHERE labels IS NOT NULL
  AND cardinality(labels) <> (SELECT count(DISTINCT x) FROM unnest(labels) AS x);
