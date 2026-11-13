-- Principals of `extra_perms` whose read grant does not include running. Empty for every
-- existing row, so every existing read grant keeps running.
ALTER TABLE script ADD COLUMN view_only TEXT[] NOT NULL DEFAULT '{}';
ALTER TABLE flow ADD COLUMN view_only TEXT[] NOT NULL DEFAULT '{}';
ALTER TABLE resource ADD COLUMN view_only TEXT[] NOT NULL DEFAULT '{}';
ALTER TABLE folder ADD COLUMN view_only TEXT[] NOT NULL DEFAULT '{}';

-- Whether any of `principals` holds a grant in `extra_perms` that includes running.
CREATE FUNCTION acl_grants_run(extra_perms JSONB, view_only TEXT[], principals TEXT[])
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM jsonb_each_text(extra_perms) e
        WHERE e.key = ANY(principals) AND (e.value::boolean OR NOT e.key = ANY(view_only))
    )
$$;
