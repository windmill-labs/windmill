-- This migration cannot be reversed.
-- The v2 compatibility layer has been permanently removed.
DO $$ BEGIN RAISE EXCEPTION 'Cannot reverse v2 finalization migration'; END $$;
