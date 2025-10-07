-- Remove step_name column
ALTER TABLE flow_conversation_message
DROP COLUMN IF EXISTS step_name;

-- Note: PostgreSQL doesn't support removing values from ENUMs directly
-- You would need to recreate the enum to remove values, which is complex
-- and may cause issues with existing data. For safety, we leave the enum values.
-- If you need to fully revert, you'll need to:
-- 1. Create a new enum without the new values
-- 2. Update all uses to the new enum
-- 3. Drop the old enum
