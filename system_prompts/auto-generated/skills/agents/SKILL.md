---
name: agents
description: MUST use when creating or changing an AI agent, the reusable `ai_agent` resource the Agents page lists.
---

# Windmill AI Agents

An agent, as the Agents page lists it, is a saved LLM with its own system prompt, tools and
memory, stored as a resource of type `ai_agent`. It is built once and used in several places: it is
run from its own page (a chat, or a run form), a flow's AI agent step links to it by path, and it
can be shared in Slack channels from its page. Build a standalone agent when the user wants an
assistant they talk to or reuse. Build a flow when the agent is one step of a larger process.

## File Format

An agent is a resource file like any other, `{path}.resource.yaml` (or `.resource.json`), with
`resource_type: ai_agent`.

## Structure

```json
{
  "resource_type": "ai_agent",
  "description": "Answers support questions from the product docs",
  "value": {
    "provider": { "kind": "openai", "resource": "$res:f/ai_providers/openai", "model": "gpt-4o" },
    "system_prompt": "You are the support assistant for Acme. Answer from the docs; say so when they don't cover a question.",
    "memory": { "kind": "compaction" },
    "output_type": "text",
    "tools": [
      {
        "id": "search_docs",
        "summary": "search_documentation",
        "description": "Search the product documentation. Use it whenever the user asks how a feature works.",
        "value": {
          "tool_type": "flowmodule",
          "type": "script",
          "path": "f/support/search_docs",
          "input_transforms": {
            "query": { "type": "ai" },
            "index": { "type": "static", "value": "product_docs" }
          }
        }
      }
    ]
  }
}
```

The agent's own settings are plain values, not the `input_transforms` an agent step in a flow uses:
each is fixed when the agent is saved. Only a tool's arguments keep `input_transforms` (see Tools). `$res:` and `$var:` references inside it are resolved at run time.

- `provider` (required): `{ "kind", "resource": "$res:<path>", "model" }`, plus an optional
  `reasoning_effort`. `resource` names the workspace's AI provider resource of that kind
- `system_prompt`: how the agent behaves. Write it for the agent's job: who it serves, what it may
  do with its tools, and what it should refuse or hand back
- `memory`: what lets the agent see earlier turns. `{ "kind": "compaction" }` keeps the whole
  conversation, summarizing its older part as it nears the model's context window; it is what a new
  agent starts with on its page. `{ "kind": "off" }` makes every run start from nothing, right for an
  agent that handles one request per run
- `output_type`: `text` (default) or `image`. Image output ignores tools and needs S3 storage on the
  workspace
- `output_schema`: a JSON schema the answer must follow, for an agent whose output a program reads
- `max_iterations`: how many model and tool rounds a run may take, 1 to 1000 (default 10)
- `streaming` (default on), `temperature`, `max_completion_tokens`: as their names say
- `run_on_behalf_of`: runs from the agent's page take the identity the agent was deployed with
  rather than the caller's, so people can use it without access to what its tools touch.
  `slack_anyone` lets anyone in the Slack channels it is shared in ask it, and is kept only with
  `run_on_behalf_of`. Set either only when the user asks: both widen who acts with the deployer's
  rights
- `on_behalf_of` is the identity itself. Deploying sets it, so never write it

## Tools

`tools` takes the same entries as a flow's AI agent step. `value.tool_type` is `flowmodule` for a
script or inline script the agent calls (or a nested agent or AI decision), `mcp` for an MCP server
(`{ "tool_type": "mcp", "resource_path": "$res:<path>", "include_tools": [], "exclude_tools": [] }`),
or `websearch` for web search (`{ "tool_type": "websearch" }`, summary `Web Search`).

- Write logic made for this agent as an inline `rawscript` tool, as a flow writes its steps. Point a
  `script` tool at a deployed script (`path`) only when one already does the job: a run calls the
  deployed version, so a script that exists only as a draft fails the run. An inline tool's
  `value` is a flow's rawscript step:
  `{ "tool_type": "flowmodule", "type": "rawscript", "language": "bun", "content": "export async function main(query: string) { ... }", "input_transforms": { "query": { "type": "ai" } } }`
- A flow cannot be a tool: a run offered one fails with `Unsupported tool`. Wrap it in a script
  that runs the flow
- A flowmodule tool's `summary` is the name the agent calls it by: letters, numbers and underscores
  only, unique among the agent's tools, and neither a reserved step id (a JavaScript keyword, `ctx`,
  `state`, `bg`) nor `__wm_web_search`. Put the explanation in `description`, which tells the agent
  when to call it
- Each argument of a tool is either the model's or fixed. `{ "type": "ai" }` (or no value) lets the
  model fill it; a static value fixes it and hides it from the model. Fix what the model must not
  choose, such as a resource, a table name or a limit

## Using It From a Flow

An AI agent step links to a saved agent with `value.agent` instead of carrying its own `provider`,
system prompt and tools:

```json
{
  "id": "support",
  "value": {
    "type": "aiagent",
    "agent": "f/support/support_agent",
    "input_transforms": {
      "user_message": { "type": "javascript", "expr": "flow_input.question" }
    },
    "tools": []
  }
}
```

The step holds only what belongs to one use of the agent: `user_message`, `user_attachments`,
`enabled_tools`, `memory_id`, `previous_messages`, and the `model` and `reasoning_effort` overrides.
Its `value.tool_inputs` (`{ "<tool id>": { "<argument>": <input transform> } }`) wires a saved
tool's arguments to this flow, e.g. `flow_input.customer_id`, over the transforms the agent saved,
without changing the agent. Everything else comes from the agent, so a change to the agent reaches
every flow linking it.
