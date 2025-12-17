# AI Provider Response Formats

This document describes the response formats for different AI providers, particularly for web search functionality.

## OpenAI Responses API

### Endpoint
`POST /responses`

### Response Format (with Web Search)

When web search is used, OpenAI returns an array of output items:

```json
{
  "output": [
    {
      "type": "web_search_call",
      "id": "ws_67c9fa0502748190b7dd390736892e100be649c1a5ff9609",
      "status": "completed"
    },
    {
      "id": "msg_67c9fa077e288190af08fdffda2e34f20be649c1a5ff9609",
      "type": "message",
      "status": "completed",
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "On March 6, 2025, several news...",
          "annotations": [
            {
              "type": "url_citation",
              "start_index": 2606,
              "end_index": 2758,
              "url": "https://...",
              "title": "Title..."
            }
          ]
        }
      ]
    }
  ]
}
```

### Output Types

1. **`web_search_call`** - Indicates web search was performed
   - `id`: Search call ID
   - `status`: "completed"

2. **`message`** - The assistant's response message
   - `role`: "assistant"
   - `status`: "completed"
   - `content`: Array of content items
     - `type`: "output_text"
     - `text`: The actual response text
     - `annotations`: Array of citations with URLs

3. **`message_call`** - Standard message (without web search)
   - `message.content`: Text or array of content parts
   - `message.tool_calls`: Array of tool calls

4. **`image_generation_call`** - For image generation
   - `result`: Base64 encoded image
   - `status`: "completed"

---

## Anthropic Messages API

### Endpoint
`POST /messages`

### Non-Streaming Response Format (with Web Search)

Anthropic returns a single message object with a content array containing multiple block types:

```json
{
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "I'll search for when Claude Shannon was born."
    },
    {
      "type": "server_tool_use",
      "id": "srvtoolu_01WYG3ziw53XMcoyKL4XcZmE",
      "name": "web_search",
      "input": {
        "query": "claude shannon birth date"
      }
    },
    {
      "type": "web_search_tool_result",
      "tool_use_id": "srvtoolu_01WYG3ziw53XMcoyKL4XcZmE",
      "content": [
        {
          "type": "web_search_result",
          "url": "https://en.wikipedia.org/wiki/Claude_Shannon",
          "title": "Claude Shannon - Wikipedia",
          "encrypted_content": "EqgfCioIARgBIiQ3YTAwMjY1Mi1mZjM5LTQ1NGUtODgxNC1kNjNjNTk1ZWI3Y...",
          "page_age": "April 30, 2025"
        }
      ]
    },
    {
      "text": "Based on the search results, ",
      "type": "text"
    },
    {
      "text": "Claude Shannon was born on April 30, 1916, in Petoskey, Michigan",
      "type": "text",
      "citations": [
        {
          "type": "web_search_result_location",
          "url": "https://en.wikipedia.org/wiki/Claude_Shannon",
          "title": "Claude Shannon - Wikipedia",
          "encrypted_index": "Eo8BCioIAhgBIiQyYjQ0OWJmZi1lNm..",
          "cited_text": "Claude Elwood Shannon (April 30, 1916 – February 24, 2001) was an American mathematician..."
        }
      ]
    }
  ],
  "id": "msg_a930390d3a",
  "usage": {
    "input_tokens": 6039,
    "output_tokens": 931,
    "server_tool_use": {
      "web_search_requests": 1
    }
  },
  "stop_reason": "end_turn"
}
```

### Content Block Types

1. **`text`** - Regular text content
   - `text`: The text content
   - `citations`: (optional) Array of citation objects for web search results

2. **`tool_use`** - User-defined tool call (should be executed by client)
   - `id`: Tool call ID
   - `name`: Tool name
   - `input`: Tool arguments as JSON object

3. **`server_tool_use`** - Server-side tool (like web_search, handled by Anthropic)
   - `id`: Tool call ID
   - `name`: Tool name (e.g., "web_search")
   - `input`: Tool arguments
   - **Note**: This is internal to Anthropic, client should NOT execute this

4. **`web_search_tool_result`** - Results from web search (internal to Anthropic)
   - `tool_use_id`: References the server_tool_use ID
   - `content`: Array of search results
   - **Note**: This is internal to Anthropic, client should NOT process this

### Streaming Response Format (SSE)

Anthropic streaming uses Server-Sent Events with these event types:

```
event: message_start
data: {"type": "message_start", "message": {...}}

event: content_block_start
data: {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}

event: content_block_delta
data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}

event: content_block_stop
data: {"type": "content_block_stop", "index": 0}

event: message_delta
data: {"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {...}}

event: message_stop
data: {"type": "message_stop"}
```

### SSE Event Types

1. **`message_start`** - Start of message
2. **`content_block_start`** - Start of a content block
   - `index`: Block index
   - `content_block`: Block type info (`text`, `tool_use`, etc.)
3. **`content_block_delta`** - Delta update for a block
   - `index`: Block index
   - `delta`: Delta content
     - `type`: "text_delta" or "input_json_delta"
     - `text`: (for text_delta) The text chunk
     - `partial_json`: (for input_json_delta) Tool arguments chunk
4. **`content_block_stop`** - End of a content block
5. **`message_delta`** - Message-level changes (stop_reason, usage)
6. **`message_stop`** - End of message
7. **`ping`** - Keep-alive ping
8. **`error`** - Error occurred

---

## Key Differences

| Aspect | OpenAI | Anthropic |
|--------|--------|-----------|
| Response structure | `output` array with multiple items | Single message with `content` array |
| Web search indicator | Separate `web_search_call` item | `server_tool_use` block in content |
| Search results | Not exposed | `web_search_tool_result` block |
| Citations | In `annotations` array | In `citations` array on text blocks |
| Tool calls | In `message.tool_calls` | `tool_use` blocks in content |
| Streaming format | OpenAI SSE with `choices[0].delta` | Anthropic SSE with `content_block_delta` |

## Implementation Notes

### What to Extract

**For OpenAI:**
- Look for `type: "message"` with `status: "completed"`
- Extract `text` from `content[].text` where `type: "output_text"`
- Skip `web_search_call` items

**For Anthropic:**
- Extract `text` from all `type: "text"` blocks
- Extract `tool_use` blocks (NOT `server_tool_use`) for tool calls
- Skip `server_tool_use` and `web_search_tool_result` - these are internal

### Streaming

**OpenAI streaming** expects:
```json
{"choices": [{"delta": {"content": "text chunk"}}]}
```

**Anthropic streaming** expects:
```json
{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "text chunk"}}
```

These are fundamentally different formats and require separate parsers.
