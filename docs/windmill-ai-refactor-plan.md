# Refactor Plan: `windmill-ai` Crate

## Context

AI provider logic is currently split across three crates with duplicate code:

- **windmill-common** — base types (`ai_types`, `ai_providers`, `ai_google`, `ai_bedrock`, `ai_cache`)
- **windmill-api** — chat proxy routes (`ai.rs`), audit logging, caching, and DB-backed credential resolution through `AIRequestConfig`
- **windmill-worker** — agent execution (`ai/` module) with `QueryBuilder` trait, SSE parsers, provider implementations

The goal: a single `windmill-ai` crate with all AI provider logic. Worker agent execution uses `QueryBuilder`; the API proxy uses `QueryBuilder::build_proxy_request` for HTTP-forwarding providers and native proxy handlers for providers that need response conversion or SDK execution.

## Dependency Direction

```
windmill-ai  →  windmill-common (for DB, Error, AgentAction, AuthedClient, etc.)
             →  windmill-types (for S3Object)
             →  windmill-parser (for Typ, used in OpenAPISchema)

windmill-api    →  windmill-ai
windmill-worker →  windmill-ai
```

windmill-common does **NOT** re-export from windmill-ai (would be circular). All consumers update imports.

## Reviewer Note: Keep API Proxy Unification Split

The crate boundary, shared utilities, SSE parsers, image handling, worker provider implementations, and provider-specific API proxy transformations are now in `windmill-ai`. The remaining duplication is credential shape and resolution: `windmill-api` still resolves DB-backed proxy credentials through `AIRequestConfig`, while worker agent execution still receives `ProviderWithResource`.

Do not jump directly from the current state to full proxy and credential unification in one PR. The API proxy combines request transformation, endpoint selection, auth headers, custom headers, OAuth user injection, Azure URL handling, Anthropic Vertex handling, Bedrock SDK calls, and SSE keepalive behavior. Split the work by risk:
- Introduce shared proxy request and credential types first.
- Move the OpenAI-compatible proxy path into `windmill-ai` next, while keeping provider-native behavior unchanged.
- Move Anthropic/Vertex, Google AI, and Bedrock in separate follow-up PRs.
- Unify credential resolution only after all proxy request builders use the shared shape.

Avoid adding modules whose only purpose is to re-export moved code. Direct imports from `windmill_ai` make ownership and dependency direction clearer at each call site.

Also do not make `build_proxy_request(raw_body, path)` too narrow. The proxy path needs method, incoming headers, resolved credentials, base URL/platform, organization/user fields, custom headers, and Bedrock/Azure/Vertex-specific context. Introduce a structured `ProxyBuildArgs`/`ProviderCredentials` shape before deleting `AIRequestConfig::prepare_request`, `google.rs`, or `bedrock.rs`.

## Completed Phase: Proxy Contract + OpenAI-Compatible Proxy ✅

Goal: introduce the shared API proxy contract in `windmill-ai` and move the OpenAI-compatible proxy request builder there without changing provider behavior.

Suggested PR title: `refactor(ai): move openai-compatible proxy building to windmill-ai`.

Scope:
- Add `windmill-ai/src/proxy.rs` and export it from `lib.rs`.
- Define `ProviderCredentials`, `ProxyBuildArgs`, and `ProxyRequest`.
- Include all context known to be needed by the current API proxy path: method, path, incoming headers, body, provider, base URL, API key, OAuth access token, organization/user fields, platform, 1M context flag, custom headers, region, and AWS credentials.
- Add a conversion from API-side `AIRequestConfig` to `ProviderCredentials`.
- Add `QueryBuilder::build_proxy_request` with a default unsupported-provider implementation.
- Implement `build_proxy_request` for OpenAI-compatible providers (`OpenAI`, `AzureOpenAI`, `Mistral`, `DeepSeek`, `Groq`, `OpenRouter`, `TogetherAI`, `CustomAI`).
- Route workspace and global API proxy requests for OpenAI-compatible providers through `windmill-ai`.
- Keep FIM transformation in `windmill-api` before calling the proxy builder.
- Keep `AIRequestConfig::prepare_request` for Anthropic/Vertex and remaining fallback paths.

Out of scope:
- Do not move Anthropic/Vertex proxy behavior yet.
- Do not move Google AI or Bedrock proxy behavior yet.
- Do not change credential resolution, audit logging, cache behavior, SSE keepalive behavior, or Bedrock/Google special cases.
- Do not remove `windmill-api/src/google.rs`, `windmill-api/src/bedrock.rs`, or `AIRequestConfig::prepare_request`.

Validation:
- `cargo test -p windmill-ai proxy`
- `cargo test -p windmill-api maps_request_config_to_provider_credentials`
- `cargo check -p windmill-ai -p windmill-api`
- `cargo check -p windmill-ai -p windmill-api --features bedrock`

Follow-up status: Anthropic/Vertex proxy handling has since moved into
`windmill-ai`, and the dead `AIRequestConfig::prepare_request` fallback has
been removed.

## Completed Phase: Proxy Execution Mode + Google AI Proxy Migration ✅

Goal: introduce a shared provider execution classifier before moving Google AI
and Bedrock. `ProxyRequest` is a good contract for HTTP-forwarding providers
such as OpenAI-compatible providers and Anthropic, but Google AI also converts
responses back to OpenAI shape and Bedrock uses SDK execution. Model that split
explicitly before moving those providers, then move the Google AI proxy
transformation into `windmill-ai` as the first native-provider migration.

Suggested PR title: `refactor(ai): add provider proxy execution mode`.

Scope:
- Add `ProxyExecutionMode` in `windmill-ai::proxy`.
- Classify providers as HTTP-forwarding, native Google AI, or native Bedrock.
- Make `supports_query_builder_proxy` derive from the shared execution mode.
- Use the shared execution mode in `windmill-api/src/ai.rs` for workspace proxy routing.
- Move Google AI workspace proxy request conversion, streaming/non-streaming response conversion, and model-list normalization into `windmill-ai::providers::google_ai`.
- Share Google AI `GeminiTextRequest` and generation-config construction between worker agent requests and API proxy requests.
- Delete the API-local `windmill-api/src/google.rs` module.
- Keep global proxy behavior, Bedrock native handling, credential resolution, audit logging, caching, and SSE keepalive behavior unchanged.

Out of scope:
- Do not move `windmill-api/src/bedrock.rs`.
- Do not unify `AIRequestConfig` and `ProviderWithResource`.

Validation:
- `cargo test -p windmill-ai google_ai`
- `cargo test -p windmill-ai proxy`
- `cargo test -p windmill-api maps_request_config_to_provider_credentials`
- `cargo test -p windmill-ai anthropic`

Follow-up status: Bedrock native proxy handling has since moved into
`windmill-ai`, and the API-local `windmill-api/src/bedrock.rs` module has been
removed.

## Current Phase PR: Bedrock Native Proxy Migration

Goal: move the remaining native-provider API proxy execution out of
`windmill-api` and into `windmill-ai`, while leaving API-owned routing,
credential resolution, auditing, cache behavior, and Axum response conversion in
`windmill-api`.

Suggested PR title: `refactor(ai): move bedrock proxy handling to windmill-ai`.

Scope:
- Move Bedrock control-plane proxy calls (`foundation-models`,
  `inference-profiles`) into `windmill-ai::providers::bedrock`.
- Move Bedrock chat proxy OpenAI request parsing, Converse request execution,
  streaming SSE conversion, non-streaming OpenAI-shaped response conversion, and
  auth selection into `windmill-ai::providers::bedrock`.
- Add an Axum-free `BedrockProxyResponse` shape in `windmill-ai`; the API route
  converts it into an Axum body.
- Move the optional `aws-sdk-bedrock` dependency from `windmill-api` to
  `windmill-ai`.
- Delete the API-local `windmill-api/src/bedrock.rs` module.

Out of scope:
- Do not unify `AIRequestConfig` and `ProviderWithResource`.
- Do not change Bedrock credential resolution, audit logging, request caching,
  or non-Bedrock proxy behavior.

Validation:
- `cargo test -p windmill-ai bedrock --features bedrock`
- `cargo check -p windmill-ai -p windmill-api`
- `cargo check -p windmill-ai -p windmill-api --features bedrock`

## Known Follow-Ups

These are not blockers for the current migration PR because they either preserve
existing behavior or need a separate product decision, but they should stay
visible for later hardening work.

- **Google AI/Gemini native proxy custom headers**: the native Google AI proxy
  path intentionally does not apply `AI_HTTP_HEADERS` or resource-level custom
  headers today. Decide whether and how env/resource custom-header injection
  should apply to Google AI once the proxy behavior is unified further.
- **Bedrock SSE tool-call indexing**: Bedrock streaming currently increments
  the OpenAI tool-call index on every Bedrock `ContentBlockStop`, including text
  content blocks. This behavior existed before the move from `windmill-api` to
  `windmill-ai`, but a later cleanup should advance the index only when the
  stopped block was a tool-use block.
- **Bedrock SSE keepalives**: Bedrock native SSE streams are still returned
  directly without the API proxy keepalive injection used by other SSE paths.
  This also preserves the pre-move behavior. A later cleanup can generalize the
  keepalive wrapper so it works for both `reqwest::Error` streams and Bedrock's
  SDK-backed `std::io::Error` streams.

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

### Step 4: Move SSE parsers to windmill-ai ✅

Move from `windmill-worker/src/ai/sse.rs` to `windmill-ai/src/sse.rs`:
- `SSEParser` trait
- `OpenAISSEParser`, `AnthropicSSEParser`, `GeminiSSEParser`, `OpenAIResponsesSSEParser`
- All associated types (delta types, usage types, etc.)

---

### Step 5: Move provider implementations to windmill-ai ✅

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

### Step 6: Move image_handler to windmill-ai ✅

Move from `windmill-worker/src/ai/image_handler.rs` to `windmill-ai/src/image_handler.rs`:
- `download_and_encode_s3_image` — no signature change needed
- `prepare_messages_for_api` — no signature change needed
- `upload_image_to_s3` — **refactor**: `(base64_image, workspace_id, job_id, client)` instead of `(base64_image, &MiniPulledJob, client)` to remove windmill-queue dependency

---

### Step 7: Move shared utilities to windmill-ai ✅

Move `AI_HTTP_HEADERS` lazy_static (currently duplicated in `windmill-api/src/ai.rs` and `windmill-worker/src/ai_executor.rs`) to `windmill_ai::utils`. Both consumers import from windmill-ai.

---

### Step 8: Add API proxy execution support to windmill-ai ✅

This is the key proxy unification step. HTTP-forwarding providers use
`QueryBuilder::build_proxy_request`:

```rust
/// Build a request from a raw OpenAI-format proxy request.
/// Used by the API chat proxy. Handles format conversion for non-OpenAI providers.
fn build_proxy_request(
    &self,
    args: &ProxyBuildArgs<'_>,
) -> Result<ProxyRequest, Error>;
```

Where `ProxyBuildArgs` carries the API proxy context that provider implementations need:
```rust
pub struct ProxyBuildArgs<'a> {
    pub method: &'a http::Method,
    pub path: &'a str,
    pub headers: &'a http::HeaderMap,
    pub body: &'a [u8],
    pub credentials: &'a ProviderCredentials,
}
```

And `ProxyRequest` contains the transformed request:
```rust
pub struct ProxyRequest {
    pub method: http::Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
```

**Provider implementations:**
- **OpenAI-compatible** (OpenAI, Mistral, DeepSeek, Groq, TogetherAI, CustomAI, OpenRouter): Minimal transformation — pass body through, build URL and auth headers.
- **Anthropic**: Handle standard vs Vertex AI. For Vertex: transform body (extract model, add anthropic_version). For standard: pass through with appropriate headers.
- **Google AI**: Native execution mode converts OpenAI format → Gemini format and Gemini responses → OpenAI shape. Replaces `windmill-api/src/google.rs`.
- **Bedrock**: Native execution mode converts OpenAI format → Bedrock SDK calls and SDK responses → OpenAI shape. Replaces `windmill-api/src/bedrock.rs`.

**Refactor API proxy** (`windmill-api/src/ai.rs`):
1. Parse provider from headers, resolve credentials → `ProviderCredentials`
2. Create `QueryBuilder` via `create_query_builder`
3. Dispatch by `ProxyExecutionMode`:
   - HTTP-forwarding providers call `query_builder.build_proxy_request(&proxy_args)` → `ProxyRequest`
   - Google AI and Bedrock call native handlers in `windmill-ai`
4. Convert the provider response to the API response body

**Remove** from windmill-api:
- `AIRequestConfig::prepare_request` — replaced by `QueryBuilder::build_proxy_request`
- `google.rs` — replaced by `windmill_ai::providers::google_ai` native proxy handlers
- `bedrock.rs` — replaced by `windmill_ai::providers::bedrock` native proxy handlers
- `transform_anthropic_for_vertex` — moved to `AnthropicQueryBuilder`
- `supports_native_fim`, `transform_fim_to_chat_completions` — moved to windmill-ai

**Keep** in API:
- `AIRequestConfig::new` credential resolution until it is refactored to produce `ProviderCredentials`
- HTTP routes, audit logging, request caching
- `inject_keepalives`, `is_sse_response` helpers
- `AIConfig`, `ExpiringAIRequestConfig` caching types

---

### Step 9: Unify credential resolution

Merge `AIRequestConfig` (API-side) and `ProviderWithResource` (worker-side) into a single credential shape in windmill-ai.

Both currently carry: api_key, base_url, region, platform, custom_headers, AWS credentials. The API's `AIRequestConfig::new` resolves credentials from DB (workspace/instance settings). The worker's `ProviderWithResource` gets credentials from the flow module definition.

Extend `windmill_ai::proxy::ProviderCredentials` as needed so both can produce it:
```rust
pub struct ProviderCredentials {
    pub provider: AIProvider,
    pub base_url: String,
    pub api_key: Option<String>,
    pub access_token: Option<String>,
    pub organization_id: Option<String>,
    pub user: Option<String>,
    pub platform: AIPlatform,
    pub region: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_session_token: Option<String>,
    pub enable_1m_context: bool,
    pub custom_headers: HashMap<String, String>,
}
```

The `create_query_builder` factory takes `&ProviderCredentials` instead of `&ProviderWithResource`.

---

## Final Crate Structure

```
windmill-ai/src/
├── lib.rs              # module exports
├── ai_types.rs         # OpenAI-compatible message types
├── ai_providers.rs     # AIProvider enum, base URLs, config
├── ai_google.rs        # Gemini types and conversions
├── ai_bedrock.rs       # Bedrock SDK wrapper (feature: bedrock)
├── ai_cache.rs         # Instance AI config revision
├── types.rs            # TokenUsage, Tool, OpenAPISchema, etc.
├── proxy.rs            # ProviderCredentials, ProxyBuildArgs, ProxyRequest
├── query_builder.rs    # QueryBuilder trait, BuildRequestArgs, ParsedResponse, StreamEventSink
├── sse.rs              # SSE parsers (OpenAI, Anthropic, Gemini, Responses)
├── image_handler.rs    # S3 image upload/download
├── utils.rs            # extract_text_content, should_use_structured_output_tool
└── providers/
    ├── mod.rs           # create_query_builder factory
    ├── anthropic.rs     # build_request + build_proxy_request
    ├── openai.rs        # build_request + build_proxy_request
    ├── google_ai.rs     # build_request + native proxy handlers
    ├── bedrock.rs       # build_request + native proxy handlers (feature: bedrock)
    ├── other.rs         # build_request + build_proxy_request
    └── openrouter.rs    # build_request + build_proxy_request
```

**windmill-worker** keeps: `ai_executor.rs`, `ai/tools.rs`, `ai/utils.rs` (flow/conversation/MCP logic), `StreamEventProcessor` (impl of `StreamEventSink`).

**windmill-api** keeps: HTTP routes (`ai.rs` proxy endpoints), audit logging, caching, credential resolution from DB. `google.rs` and `bedrock.rs` deleted.
