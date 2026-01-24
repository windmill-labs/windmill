-- Step 1: Add error_handler_muted_on_user_path column if it doesn't exist (was previously a separate migration)
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS error_handler_muted_on_user_path BOOL NOT NULL DEFAULT false;

-- Step 2: Add new JSONB columns with temporary names (to avoid collision with existing TEXT columns)
ALTER TABLE workspace_settings
  ADD COLUMN IF NOT EXISTS auto_invite JSONB,
  ADD COLUMN IF NOT EXISTS error_handler_new JSONB,
  ADD COLUMN IF NOT EXISTS success_handler_new JSONB;

-- Step 3: Migrate existing data to new columns (use jsonb_strip_nulls to omit null values)
UPDATE workspace_settings SET
  auto_invite = jsonb_strip_nulls(jsonb_build_object(
    'enabled', auto_invite_domain IS NOT NULL,
    'domain', auto_invite_domain,
    'operator', COALESCE(auto_invite_operator, false),
    'mode', CASE WHEN auto_add THEN 'add' ELSE 'invite' END,
    'instance_groups', auto_add_instance_groups,
    'instance_groups_roles', auto_add_instance_groups_roles
  )),
  error_handler_new = CASE
    WHEN error_handler IS NOT NULL THEN jsonb_strip_nulls(jsonb_build_object(
      'path', error_handler,
      'extra_args', error_handler_extra_args::jsonb,
      'muted_on_cancel', error_handler_muted_on_cancel,
      'muted_on_user_path', error_handler_muted_on_user_path
    ))
    ELSE NULL
  END,
  success_handler_new = CASE
    WHEN success_handler IS NOT NULL THEN jsonb_strip_nulls(jsonb_build_object(
      'path', success_handler,
      'extra_args', success_handler_extra_args::jsonb
    ))
    ELSE NULL
  END;

-- Step 4: Drop old columns that are now consolidated
ALTER TABLE workspace_settings
  DROP COLUMN IF EXISTS auto_invite_domain,
  DROP COLUMN IF EXISTS auto_invite_operator,
  DROP COLUMN IF EXISTS auto_add,
  DROP COLUMN IF EXISTS auto_add_instance_groups,
  DROP COLUMN IF EXISTS auto_add_instance_groups_roles,
  DROP COLUMN IF EXISTS error_handler,
  DROP COLUMN IF EXISTS error_handler_extra_args,
  DROP COLUMN IF EXISTS error_handler_muted_on_cancel,
  DROP COLUMN IF EXISTS error_handler_muted_on_user_path,
  DROP COLUMN IF EXISTS success_handler,
  DROP COLUMN IF EXISTS success_handler_extra_args;

-- Step 5: Rename new columns to their final names
ALTER TABLE workspace_settings
  RENAME COLUMN error_handler_new TO error_handler;
ALTER TABLE workspace_settings
  RENAME COLUMN success_handler_new TO success_handler;
