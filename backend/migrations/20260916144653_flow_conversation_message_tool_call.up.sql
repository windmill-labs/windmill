-- A chat is rebuilt from its rows without reading jobs. A tool row for a script or flow
-- names the tool's own job, which holds its call. An MCP tool runs inside the agent's job,
-- whose result lists every call of the turn with nothing tying one to a row, so its row
-- carries the call itself; a provider-native web search carries only its citations, the
-- provider never returning the query.
ALTER TABLE flow_conversation_message ADD COLUMN tool_arguments TEXT;
ALTER TABLE flow_conversation_message ADD COLUMN tool_result TEXT;

-- The thinking behind this row. The agent job keeps the turn's thinking as one string;
-- the rows keep it per iteration, next to the answer or tool call it led to.
ALTER TABLE flow_conversation_message ADD COLUMN reasoning TEXT;
