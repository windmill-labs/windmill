-- AI chat usage telemetry now flows through the generic feature_usage table
-- (ai_chat/message and ai_chat/model events).
DROP TABLE ai_chat_usage;
