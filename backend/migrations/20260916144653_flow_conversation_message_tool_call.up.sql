-- A chat is rebuilt from its rows without reading jobs, so every tool row carries its call:
-- the arguments the model wrote and the text the model got back, or what the call failed
-- with. A script or flow tool's job holds the args its input transforms produced, not the
-- model's; an MCP tool runs inside the agent's job, whose result lists every call of the
-- turn with nothing tying one to a row. A provider-native web search carries only its
-- citations, the provider never returning the query.
ALTER TABLE flow_conversation_message ADD COLUMN tool_arguments TEXT;
ALTER TABLE flow_conversation_message ADD COLUMN tool_result TEXT;

-- The thinking behind this row. The agent job keeps the turn's thinking as one string;
-- the rows keep it per iteration, next to the answer or tool call it led to.
ALTER TABLE flow_conversation_message ADD COLUMN reasoning TEXT;
