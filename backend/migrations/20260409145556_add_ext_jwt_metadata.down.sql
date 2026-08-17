-- Remove metadata columns from unique_ext_jwt_token

ALTER TABLE unique_ext_jwt_token
  DROP COLUMN IF EXISTS email,
  DROP COLUMN IF EXISTS username,
  DROP COLUMN IF EXISTS is_admin,
  DROP COLUMN IF EXISTS is_operator,
  DROP COLUMN IF EXISTS workspace_id,
  DROP COLUMN IF EXISTS label,
  DROP COLUMN IF EXISTS scopes;
