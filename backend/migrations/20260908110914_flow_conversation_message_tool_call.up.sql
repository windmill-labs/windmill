-- A tool row's call and result are read back from the tool's own job, which an MCP tool
-- and a provider-native tool never have: they run inside the agent's job. For those the
-- row is the only record, so it carries the call itself.
ALTER TABLE flow_conversation_message ADD COLUMN tool_arguments TEXT;
ALTER TABLE flow_conversation_message ADD COLUMN tool_result TEXT;

-- The thinking that produced an answer is streamed, never returned in the response body,
-- so it exists nowhere once the stream is over.
ALTER TABLE flow_conversation_message ADD COLUMN reasoning TEXT;
