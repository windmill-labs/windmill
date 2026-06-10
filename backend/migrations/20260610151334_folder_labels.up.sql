-- Add up migration script here
ALTER TABLE folder ADD COLUMN labels text[];

-- Returns the labels of the folder containing the given item path ('f/<folder>/...').
-- SECURITY DEFINER so it bypasses folder RLS: inherited labels must be consistent
-- regardless of who reads the item or pushes the job (a user can have access to an
-- item without being in the folder's extra_perms, and label values are already
-- workspace-visible via the labels/list endpoint).
CREATE FUNCTION folder_labels(w_id text, item_path text) RETURNS text[]
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public AS $$
    SELECT labels FROM folder
    WHERE workspace_id = w_id AND item_path LIKE 'f/%' AND name = split_part(item_path, '/', 2)
$$;
