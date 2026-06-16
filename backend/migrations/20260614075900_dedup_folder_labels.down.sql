-- Restore the original passthrough definition (the one-time data cleanup is not reverted).
CREATE OR REPLACE FUNCTION folder_labels(w_id text, item_path text) RETURNS text[]
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public AS $$
    SELECT labels FROM folder
    WHERE workspace_id = w_id AND item_path LIKE 'f/%' AND name = split_part(item_path, '/', 2)
$$;
