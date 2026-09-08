-- Rebuild email from permissioned_as, mirroring
-- windmill_common::users::get_email_from_permissioned_as.

ALTER TABLE azure_trigger ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';

UPDATE azure_trigger t SET email = CASE
    WHEN t.permissioned_as LIKE 'u/%' THEN COALESCE(
        (SELECT u.email FROM usr u
          WHERE u.workspace_id = t.workspace_id
            AND u.username = SUBSTRING(t.permissioned_as FROM 3)),
        (SELECT p.email FROM password p
          WHERE p.super_admin
            AND (p.username = SUBSTRING(t.permissioned_as FROM 3)
                 OR p.email = SUBSTRING(t.permissioned_as FROM 3))
          LIMIT 1),
        SUBSTRING(t.permissioned_as FROM 3) || '@unknown.windmill.dev'
    )
    WHEN t.permissioned_as LIKE 'g/%'
        THEN 'group-' || SUBSTRING(t.permissioned_as FROM 3) || '@windmill.dev'
    ELSE t.permissioned_as
END;

ALTER TABLE azure_trigger ALTER COLUMN email DROP DEFAULT;
