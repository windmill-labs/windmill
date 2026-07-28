-- AI chat usage telemetry now flows through the generic feature_usage table
-- (ai_chat/message and ai_chat/model events). Backfill the accumulated rows so
-- no reporting window is lost, then drop the old table. Day-bucketing uses the
-- chat's first-message date; values are filtered to the identifier shape the
-- logging endpoint enforces.
INSERT INTO feature_usage (feature, kind, key, entity_id, day, value, updated_at)
SELECT 'ai_chat', 'message', mode, session_id, created_at::date, message_count, created_at
FROM ai_chat_usage
WHERE mode ~ '^[A-Za-z0-9_:./-]{1,100}$'
  AND session_id ~ '^[A-Za-z0-9_:./-]{1,50}$'
ON CONFLICT (feature, kind, key, entity_id, day)
DO UPDATE SET value = feature_usage.value + EXCLUDED.value;

INSERT INTO feature_usage (feature, kind, key, entity_id, day, value, updated_at)
SELECT 'ai_chat', 'model', provider || ':' || model, session_id, created_at::date, message_count, created_at
FROM ai_chat_usage
WHERE (provider || ':' || model) ~ '^[A-Za-z0-9_:./-]{1,100}$'
  AND session_id ~ '^[A-Za-z0-9_:./-]{1,50}$'
ON CONFLICT (feature, kind, key, entity_id, day)
DO UPDATE SET value = feature_usage.value + EXCLUDED.value;

DROP TABLE ai_chat_usage;
