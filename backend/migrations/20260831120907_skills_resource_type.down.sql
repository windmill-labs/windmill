-- The up migration only ever added: `ai_skill` still holds every skill it copied,
-- so there is nothing to restore and nothing to delete. Removing the resources
-- would destroy any a user has since edited or created, and removing a folder
-- would take whatever else was put in it.
--
-- The seeded resource type goes. `created_by` only distinguishes this migration's
-- row from one a user created by hand: a hub sync updates the schema in place and
-- leaves `created_by` alone, so a synced-over row is still removed here and the
-- next sync puts it back.
DELETE FROM resource_type
WHERE workspace_id = 'admins'
  AND name = 'ai_skill'
  AND created_by = 'system';
