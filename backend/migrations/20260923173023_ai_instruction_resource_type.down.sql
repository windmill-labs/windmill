-- `created_by` tells this migration's row from one a user created by hand; a hub
-- sync updates in place and leaves it alone.
DELETE FROM resource_type
WHERE workspace_id = 'admins'
  AND name = 'ai_instruction'
  AND created_by = 'system';
