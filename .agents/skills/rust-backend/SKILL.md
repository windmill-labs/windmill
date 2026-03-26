---
name: rust-backend
description: Rust coding guidelines for the Windmill backend. MUST use when writing or modifying Rust code in the backend directory.
---

# Windmill Rust Patterns

Apply these Windmill-specific patterns when writing Rust code in `backend/`.

## Error Handling

Use `Error` from `windmill_common::error`. Return `Result<T, Error>` or `JsonResult<T>`:

```rust
use windmill_common::error::{Error, Result};

pub async fn get_job(db: &DB, id: Uuid) -> Result<Job> {
    sqlx::query_as!(Job, "SELECT id, workspace_id FROM v2_job WHERE id = $1", id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| Error::NotFound("job not found".to_string()))?;
}
```

Never panic in library code. Reserve `.unwrap()` for compile-time guarantees.

## SQLx Patterns

**Never use `SELECT *`** — always list columns explicitly. Critical for backwards compatibility when workers lag behind API version:

```rust
// Correct
sqlx::query_as!(Job, "SELECT id, workspace_id, path FROM v2_job WHERE id = $1", id)

// Wrong — breaks when columns are added
sqlx::query_as!(Job, "SELECT * FROM v2_job WHERE id = $1", id)
```

Use batch operations to avoid N+1:

```rust
// Preferred — single query with IN clause
sqlx::query!("SELECT ... WHERE id = ANY($1)", &ids[..]).fetch_all(db).await?
```

Use transactions for multi-step operations. Parameterize all queries.

## JSON Handling

Prefer `Box<serde_json::value::RawValue>` over `serde_json::Value` when storing/passing JSON without inspection:

```rust
pub struct Job {
    pub args: Option<Box<serde_json::value::RawValue>>,
}
```

Only use `serde_json::Value` when you need to inspect or modify the JSON.

## Serde Optimizations

```rust
#[derive(Serialize, Deserialize)]
pub struct Job {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<Uuid>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: i32,
}
```

## Async & Concurrency

Never block the async runtime. Use `spawn_blocking` for CPU-intensive work:

```rust
let result = tokio::task::spawn_blocking(move || expensive_computation(&data)).await?;
```

**Mutex selection**: Prefer `std::sync::Mutex` (or `parking_lot::Mutex`) for data protection. Only use `tokio::sync::Mutex` when holding locks across `.await` points.

Use `tokio::sync::mpsc` (bounded) for channels. Avoid `std::thread::sleep` in async contexts.

## Module Structure & Visibility

- Use `pub(crate)` instead of `pub` when possible
- Place new code in the appropriate crate based on functionality
- API endpoints go in `windmill-api/src/` organized by domain
- Shared functionality goes in `windmill-common/src/`

## Code Navigation

Always use rust-analyzer LSP for go-to-definition, find-references, and type info. Do not guess at module paths.

## Axum Handlers

Destructure extractors directly in function signatures:

```rust
async fn process_job(
    Extension(db): Extension<DB>,
    Path((workspace, job_id)): Path<(String, Uuid)>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Job>> { ... }
```
