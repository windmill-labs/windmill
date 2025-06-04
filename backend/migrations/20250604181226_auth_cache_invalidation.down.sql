
-- Remove token invalidation notification trigger

DROP TRIGGER IF EXISTS token_invalidation_trigger ON token;
DROP FUNCTION IF EXISTS notify_token_invalidation();
