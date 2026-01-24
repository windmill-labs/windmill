-- Step 1: Rename current JSONB columns to temporary names to avoid collision
ALTER TABLE workspace_settings
  RENAME COLUMN error_handler TO error_handler_jsonb;
ALTER TABLE workspace_settings
  RENAME COLUMN success_handler TO success_handler_jsonb;

-- Step 2: Restore old columns with original names and types
ALTER TABLE workspace_settings
  ADD COLUMN IF NOT EXISTS auto_invite_domain VARCHAR(255),
  ADD COLUMN IF NOT EXISTS auto_invite_operator BOOLEAN,
  ADD COLUMN IF NOT EXISTS auto_add BOOLEAN,
  ADD COLUMN IF NOT EXISTS auto_add_instance_groups TEXT[],
  ADD COLUMN IF NOT EXISTS auto_add_instance_groups_roles JSONB,
  ADD COLUMN IF NOT EXISTS error_handler VARCHAR(255),
  ADD COLUMN IF NOT EXISTS error_handler_extra_args JSON,
  ADD COLUMN IF NOT EXISTS error_handler_muted_on_cancel BOOLEAN DEFAULT false,
  ADD COLUMN IF NOT EXISTS error_handler_muted_on_user_path BOOLEAN DEFAULT false,
  ADD COLUMN IF NOT EXISTS success_handler TEXT,
  ADD COLUMN IF NOT EXISTS success_handler_extra_args JSON;

-- Step 3: Migrate data back from JSONB columns to flat columns
UPDATE workspace_settings SET
  auto_invite_domain = auto_invite->>'domain',
  auto_invite_operator = (auto_invite->>'as') = 'operator',
  auto_add = (auto_invite->>'mode') = 'add',
  auto_add_instance_groups = CASE
    WHEN auto_invite->'instance_groups' IS NOT NULL AND auto_invite->'instance_groups' != 'null'::jsonb
    THEN ARRAY(SELECT jsonb_array_elements_text(auto_invite->'instance_groups'))
    ELSE NULL
  END,
  auto_add_instance_groups_roles = auto_invite->'instance_groups_roles',
  error_handler = error_handler_jsonb->>'path',
  error_handler_extra_args = (error_handler_jsonb->'extra_args')::json,
  error_handler_muted_on_cancel = COALESCE((error_handler_jsonb->>'muted_on_cancel')::boolean, false),
  error_handler_muted_on_user_path = COALESCE((error_handler_jsonb->>'muted_on_user_path')::boolean, false),
  success_handler = success_handler_jsonb->>'path',
  success_handler_extra_args = (success_handler_jsonb->'extra_args')::json;

-- Step 4: Drop the JSONB columns
ALTER TABLE workspace_settings
  DROP COLUMN IF EXISTS auto_invite,
  DROP COLUMN IF EXISTS error_handler_jsonb,
  DROP COLUMN IF EXISTS success_handler_jsonb;
