-- Add down migration script here
-- Restores the column and re-derives it from the principal. Rows whose principal names a
-- group get that group's synthetic address, which is what the column held before.
ALTER TABLE script ADD COLUMN IF NOT EXISTS on_behalf_of_email TEXT;
ALTER TABLE flow ADD COLUMN IF NOT EXISTS on_behalf_of_email TEXT;

CREATE FUNCTION pg_temp.email_from_permissioned_as(w_id VARCHAR, permissioned_as VARCHAR)
RETURNS VARCHAR AS $$
    SELECT CASE
        WHEN $2 LIKE 'g/%' AND $2 NOT LIKE '%@%'
            THEN 'group-' || substring($2 from 3) || '@windmill.dev'
        WHEN $2 LIKE 'u/%' AND $2 NOT LIKE '%@%'
            THEN COALESCE(
                (SELECT u.email FROM usr u WHERE u.workspace_id = $1 AND u.username = substring($2 from 3)),
                substring($2 from 3) || '@unknown.windmill.dev')
        ELSE $2
    END;
$$ LANGUAGE SQL STABLE;

UPDATE script SET on_behalf_of_email =
    pg_temp.email_from_permissioned_as(workspace_id, on_behalf_of_permissioned_as)
 WHERE on_behalf_of_permissioned_as IS NOT NULL;

UPDATE flow SET on_behalf_of_email =
    pg_temp.email_from_permissioned_as(workspace_id, on_behalf_of_permissioned_as)
 WHERE on_behalf_of_permissioned_as IS NOT NULL;
