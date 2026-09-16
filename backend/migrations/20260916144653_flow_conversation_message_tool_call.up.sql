-- A chat is rebuilt from its rows without reading jobs. A tool row for a script or flow
-- names the tool's own job, which holds its call; an MCP tool and a provider-native tool
-- run inside the agent's job, whose result lists every call of the turn with nothing tying
-- one to a row. For those the row carries the call itself.
ALTER TABLE flow_conversation_message ADD COLUMN tool_arguments TEXT;
ALTER TABLE flow_conversation_message ADD COLUMN tool_result TEXT;

-- The thinking behind this row. The agent job keeps the turn's thinking as one string;
-- the rows keep it per iteration, next to the answer or tool call it led to.
ALTER TABLE flow_conversation_message ADD COLUMN reasoning TEXT;
