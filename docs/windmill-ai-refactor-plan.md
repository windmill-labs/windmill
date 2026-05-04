# Refactor Plan: `windmill-ai` Crate

## Context

AI provider logic is currently split across three crates with duplicate code:

- **windmill-common** — base types (`ai_types`, `ai_providers`, `ai_google`, `ai_bedrock`, `ai_cache`)
- **windmill-api** — chat proxy (`ai.rs`, `google.rs`, `bedrock.rs`) with its own request building for Google/Bedrock, plus `AIRequestConfig::prepare_request` for auth/URL handling
- **windmill-worker** — agent execution (`ai/` module) with `QueryBuilder` trait, SSE parsers, provider implementations

The goal: a single `windmill-ai` crate with all AI provider logic. Both the API proxy and worker agent use `QueryBuilder` for every provider — no more duplicate logic.

## Dependency Direction

```
windmill-ai  →  windmill-common (for DB, Error, AgentAction, AuthedClient, etc.)
             →  windmill-types (for S3Object)
             →  windmill-parser (for Typ, used in OpenAPISchema)

windmill-api    →  windmill-ai
windmill-worker →  windmill-ai
```

windmill-common does **NOT** re-export from windmill-ai (would be circular). All consumers update imports.

## Step-by-Step Plan

Each step produces a compiling, working backend.

---

### Step 1: Create `windmill-ai` crate, move base types from windmill-common ✅

Create `backend/windmill-ai/Cargo.toml` and `backend/windmill-ai/src/lib.rs`.

Move from `windmill-common/src/` to `windmill-ai/src/`:
- `ai_types.rs` — OpenAI-compatible message types
- `ai_providers.rs` — `AIProvider` enum, `AIPlatform`, base URLs, `ProviderConfig`
- `ai_google.rs` — Gemini types and OpenAI↔Gemini conversion
- `ai_bedrock.rs` — Bedrock SDK wrapper (feature-gated on `bedrock`)
- `ai_cache.rs` — instance AI config revision tracking

Update all imports (`windmill_common::ai_*` → `windmill_ai::ai_*`).

---

### Step 2: Move worker AI types to windmill-ai ✅

Move from `windmill-worker/src/ai/types.rs` to `windmill-ai/src/types.rs`:
- `ProviderWithResource`, `ProviderResource` — credential types
- `TokenUsage` — token usage tracking
- `OutputType`, `SchemaType`, `AdditionalProperties` — output configuration
- `OpenAPISchema` — tool parameter schema (depends on `windmill-parser::Typ`)
- `Tool`, `Message`, `ResponseFormat`, `JsonSchemaFormat` — agent types
- `StreamingEvent` — SSE event enum
- `AIAgentArgs`, `AIAgentArgsRaw`, `AIAgentResult` — agent job args
- `Memory` — agent memory enum
- `S3ObjectWithType` — S3 image type
- `McpToolSource` stub (with same `#[cfg(feature = "mcp")]` pattern)

Worker `ai/types.rs` becomes a re-export: `pub use windmill_ai::types::*`.

---

### Step 3: Move QueryBuilder trait, ParsedResponse, and StreamEventSink abstraction to windmill-ai ✅

Move from `windmill-worker/src/ai/query_builder.rs` to `windmill-ai/src/query_builder.rs`:
- `BuildRequestArgs` struct
- `ParsedResponse` enum
- `QueryBuilder` trait (with all existing methods)

New `StreamEventSink` trait in windmill-ai:
```rust
#[async_trait]
pub trait StreamEventSink: Send + Sync {
    async fn send(&self, event: StreamingEvent, events_str: &mut String) -> Result<(), Error>;
}
```

`StreamEventSink` abstracts the worker's `StreamEventProcessor` so windmill-ai doesn't depend on windmill-queue or the worker's job logger. The worker's `StreamEventProcessor` implements `StreamEventSink`. All provider `parse_streaming_response` methods and SSE parsers accept `Box<dyn StreamEventSink>`.

---

### Step 4: Move SSE parsers to windmill-ai

Move from `windmill-worker/src/ai/sse.rs` to `windmill-ai/src/sse.rs`:
- `SSEParser` trait
- `OpenAISSEParser`, `AnthropicSSEParser`, `GeminiSSEParser`, `OpenAIResponsesSSEParser`
- All associated types (delta types, usage types, etc.)

---

### Step 5: Move provider implementations to windmill-ai

Move from `windmill-worker/src/ai/providers/` to `windmill-ai/src/providers/`:
- `anthropic.rs` — `AnthropicQueryBuilder`
- `openai.rs` — `OpenAIQueryBuilder`
- `google_ai.rs` — `GoogleAIQueryBuilder`
- `bedrock.rs` — `BedrockQueryBuilder` (feature-gated)
- `other.rs` — `OtherQueryBuilder` (Mistral, DeepSeek, Groq, TogetherAI, CustomAI)
- `openrouter.rs` — `OpenRouterQueryBuilder`
- `mod.rs` with `create_query_builder` factory

Move utility functions providers depend on:
- `should_use_structured_output_tool` (from `utils.rs`)
- `extract_text_content` (from `utils.rs`)

---

### Step 6: Move image_handler to windmill-ai

Move from `windmill-worker/src/ai/image_handler.rs` to `windmill-ai/src/image_handler.rs`:
- `download_and_encode_s3_image` — no signature change needed
- `prepare_messages_for_api` — no signature change needed
- `upload_image_to_s3` — **refactor**: `(base64_image, workspace_id, job_id, client)` instead of `(base64_image, &MiniPulledJob, client)` to remove windmill-queue dependency

---

### Step 7: Move shared utilities to windmill-ai

Move `AI_HTTP_HEADERS` lazy_static (currently duplicated in `windmill-api/src/ai.rs` and `windmill-worker/src/ai_executor.rs`) to `windmill_ai::utils`. Both consumers import from windmill-ai.

---

### Step 8: Add proxy support to QueryBuilder — API uses QueryBuilder for all providers

This is the key unification step. Add a new method to the `QueryBuilder` trait:

```rust
/// Build a request from a raw OpenAI-format proxy request.
/// Used by the API chat proxy. Handles format conversion for non-OpenAI providers.
fn build_proxy_request(
    &self,
    raw_body: &[u8],
    path: &str,
) -> Result<ProxyRequest, Error>;
```

Where `ProxyRequest` contains the transformed body, endpoint URL, and auth headers:
```rust
pub struct ProxyRequest {
    pub url: String,
    pub body: Vec<u8>,
    pub auth_headers: Vec<(String, String)>,
    pub is_sse: bool,
}
```

**Provider implementations:**
- **OpenAI-compatible** (OpenAI, Mistral, DeepSeek, Groq, TogetherAI, CustomAI, OpenRouter): Minimal transformation — pass body through, build URL and auth headers.
- **Anthropic**: Handle standard vs Vertex AI. For Vertex: transform body (extract model, add anthropic_version). For standard: pass through with appropriate headers.
- **Google AI**: Convert OpenAI format → Gemini format (using existing `ai_google` functions). Replaces `windmill-api/src/google.rs`.
- **Bedrock**: Convert OpenAI format → Bedrock SDK calls. Replaces `windmill-api/src/bedrock.rs`.

**Refactor API proxy** (`windmill-api/src/ai.rs`):
1. Parse provider from headers, resolve credentials → `ProviderWithResource`
2. Create `QueryBuilder` via `create_query_builder`
3. Call `query_builder.build_proxy_request(body, path)` → `ProxyRequest`
4. Send the request, return response with SSE keepalive injection

**Remove** from windmill-api:
- `AIRequestConfig::prepare_request` — replaced by `QueryBuilder::build_proxy_request`
- `google.rs` — replaced by `GoogleAIQueryBuilder::build_proxy_request`
- `bedrock.rs` — replaced by `BedrockQueryBuilder::build_proxy_request`
- `transform_anthropic_for_vertex` — moved to `AnthropicQueryBuilder`
- `supports_native_fim`, `transform_fim_to_chat_completions` — moved to windmill-ai

**Keep** in API:
- `AIRequestConfig::new` credential resolution (or refactor to produce `ProviderWithResource`)
- HTTP routes, audit logging, request caching
- `inject_keepalives`, `is_sse_response` helpers
- `AIConfig`, `ExpiringAIRequestConfig` caching types

---

### Step 9: Unify credential resolution

Merge `AIRequestConfig` (API-side) and `ProviderWithResource` (worker-side) into a single type in windmill-ai.

Both currently carry: api_key, base_url, region, platform, custom_headers, AWS credentials. The API's `AIRequestConfig::new` resolves credentials from DB (workspace/instance settings). The worker's `ProviderWithResource` gets credentials from the flow module definition.

Create `windmill_ai::ProviderCredentials` that both can produce:
```rust
pub struct ProviderCredentials {
    pub provider: AIProvider,
    pub api_key: Option<String>,
    pub base_url: String,
    pub platform: AIPlatform,
    pub region: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_session_token: Option<String>,
    pub custom_headers: HashMap<String, String>,
    pub organization_id: Option<String>,
    pub enable_1m_context: bool,
}
```

The `create_query_builder` factory takes `&ProviderCredentials` instead of `&ProviderWithResource`.

---

## Final Crate Structure

```
windmill-ai/src/
├── lib.rs              # re-exports, AI_HTTP_HEADERS
├── ai_types.rs         # OpenAI-compatible message types
├── ai_providers.rs     # AIProvider enum, base URLs, config
├── ai_google.rs        # Gemini types and conversions
├── ai_bedrock.rs       # Bedrock SDK wrapper (feature: bedrock)
├── ai_cache.rs         # Instance AI config revision
├── types.rs            # ProviderCredentials, TokenUsage, Tool, OpenAPISchema, etc.
├── query_builder.rs    # QueryBuilder trait, BuildRequestArgs, ParsedResponse, ProxyRequest, StreamEventSink
├── sse.rs              # SSE parsers (OpenAI, Anthropic, Gemini, Responses)
├── image_handler.rs    # S3 image upload/download
├── utils.rs            # extract_text_content, should_use_structured_output_tool
└── providers/
    ├── mod.rs           # create_query_builder factory
    ├── anthropic.rs     # build_request + build_proxy_request
    ├── openai.rs        # build_request + build_proxy_request
    ├── google_ai.rs     # build_request + build_proxy_request
    ├── bedrock.rs       # build_request + build_proxy_request (feature: bedrock)
    ├── other.rs         # build_request + build_proxy_request
    └── openrouter.rs    # build_request + build_proxy_request
```

**windmill-worker** keeps: `ai_executor.rs`, `ai/tools.rs`, `ai/utils.rs` (flow/conversation/MCP logic), `StreamEventProcessor` (impl of `StreamEventSink`).

**windmill-api** keeps: HTTP routes (`ai.rs` proxy endpoints), audit logging, caching, credential resolution from DB. `google.rs` and `bedrock.rs` deleted.
