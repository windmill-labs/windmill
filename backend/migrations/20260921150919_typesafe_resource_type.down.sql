-- `created_by` tells this migration's row from one a user created by hand. Resources of the type
-- are left alone: they hold credentials a user entered.
DELETE FROM resource_type
WHERE workspace_id = 'admins'
  AND name = 'typesafe'
  AND created_by = 'system';
