-- The up migration only ever added: `ai_skill` still holds every skill it copied,
-- so there is nothing to restore and nothing to delete. Removing the resources
-- would destroy any a user has since edited or created, and removing a folder
-- would take whatever else was put in it.
--
-- The seeded resource type goes, and only when it is still the seeded one — an
-- instance that has since synced the type from the hub owns that row.
DELETE FROM resource_type
WHERE workspace_id = 'admins'
  AND name = 'ai_skill'
  AND created_by = 'system';
