---
name: rust-backend
description: Rust coding guidelines for the Windmill backend. Apply when writing or modifying Rust code in the backend directory.
---

# Rust Backend Coding Guidelines

Apply these patterns when writing or modifying Rust code in the `backend/` directory.

## Data Structure Design

Choose between `struct`, `enum`, or `newtype` based on domain needs:

- Use `enum` for state machines instead of boolean flags or loosely related fields
- Model invariants explicitly using types (e.g., `NonZeroU32`, `Duration`, custom enums)
- Consider ownership of each field:
  - Use `&str` vs `String`, slices vs vectors
  - Use `Arc<T>` when sharing across threads
  - Use `Cow<'a, T>` for flexible ownership

```rust
// State machine with enum
enum JobState {
    Pending { scheduled_for: DateTime<Utc> },
    Running { started_at: DateTime<Utc>, worker: String },
    Completed { result: JobResult, duration_ms: i64 },
    Failed { error: String, retries: u32 },
}

// Avoid multiple booleans
struct Job {
    is_pending: bool,   // Don't do this
    is_running: bool,
    is_completed: bool,
}
```

## Impl Block Organization

Place `impl` blocks immediately below the struct/enum they modify. Group methods logically:

```rust
struct JobQueue {
    jobs: Vec<Job>,
    capacity: usize,
}

impl JobQueue {
    // Constructors first
    pub fn new(capacity: usize) -> Self { ... }
    pub fn with_jobs(jobs: Vec<Job>) -> Self { ... }

    // Getters
    pub fn len(&self) -> usize { ... }
    pub fn is_empty(&self) -> bool { ... }

    // Mutation methods
    pub fn push(&mut self, job: Job) -> Result<()> { ... }
    pub fn pop(&mut self) -> Option<Job> { ... }

    // Domain logic
    pub fn next_scheduled(&self) -> Option<&Job> { ... }
}
```

## Iterator Chains Over For-Loops

Prefer functional iterator chains (`.filter().map().collect()`) over imperative for-loops:

```rust
// Preferred
let results: Vec<_> = items
    .iter()
    .filter(|item| item.is_valid())
    .map(|item| item.transform())
    .collect();

// Avoid
let mut results = Vec::new();
for item in items.iter() {
    if item.is_valid() {
        results.push(item.transform());
    }
}
```

## Error Handling

Use the `Error` type from `windmill_common::error`. Return `Result<T, Error>` or `JsonResult<T>` for fallible functions:

```rust
use windmill_common::error::{Error, Result};

// Use ? operator for propagation
pub async fn get_job(db: &DB, id: Uuid) -> Result<Job> {
    let job = sqlx::query_as!(Job, "SELECT ... WHERE id = $1", id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| Error::NotFound("job not found".to_string()))?;
    Ok(job)
}
```

Prefer `if let` for optional handling. Use `let...else` when early return makes code clearer:

```rust
let Some(config) = get_config() else {
    return Err(Error::MissingConfig);
};
```

Never panic in library code. Reserve `.unwrap()` for cases with compile-time guarantees. Keep functions short to help lifetime inference and clarity.

## Early Returns

Return early to avoid deep nesting. Handle error cases and edge conditions first:

```rust
// Preferred - early returns
fn process_job(job: Option<Job>) -> Result<Output> {
    let Some(job) = job else {
        return Ok(Output::default());
    };

    if !job.is_valid() {
        return Err(Error::InvalidJob);
    }

    if job.is_cached() {
        return Ok(job.cached_result());
    }

    // Main logic at the end, not nested
    execute_job(job)
}

// Avoid - deep nesting
fn process_job(job: Option<Job>) -> Result<Output> {
    if let Some(job) = job {
        if job.is_valid() {
            if !job.is_cached() {
                execute_job(job)
            } else {
                Ok(job.cached_result())
            }
        } else {
            Err(Error::InvalidJob)
        }
    } else {
        Ok(Output::default())
    }
}
```

## Variable Shadowing

Shadow variables instead of creating new names with prefixes:

```rust
// Preferred
let data = fetch_raw_data();
let data = parse(data);
let data = validate(data)?;

// Avoid
let raw_data = fetch_raw_data();
let parsed_data = parse(raw_data);
let validated_data = validate(parsed_data)?;
```

## Minimal Comments

- No inline comments explaining obvious code
- No TODO/FIXME comments in committed code
- Doc comments (`///`) only on public items
- Let code be self-documenting through clear naming

## Type Safety

Use enums over boolean flags for clarity:

```rust
// Preferred
enum JobStatus {
    Pending,
    Running,
    Completed,
}

// Avoid
struct Job {
    is_running: bool,
    is_completed: bool,
}
```

## Pattern Matching

Prefer explicit matching. Use wildcards strategically for fallback cases or ignored fields:

```rust
// Explicit matching preferred
match status {
    JobStatus::Pending => handle_pending(),
    JobStatus::Running => handle_running(),
    JobStatus::Completed => handle_completed(),
}

// Wildcards OK for fallback
match result {
    Ok(value) => process(value),
    Err(_) => return default_value(),
}

// Wildcards OK for ignoring fields in destructuring
let Point { x, y, .. } = point;
```

## Destructuring in Function Signatures

Destructure structs directly in function parameters:

```rust
// Preferred
async fn process_job(
    Extension(db): Extension<DB>,
    Path((workspace, job_id)): Path<(String, Uuid)>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Job>> {
    // ...
}

// Avoid
async fn process_job(
    db_ext: Extension<DB>,
    path: Path<(String, Uuid)>,
    query: Query<Pagination>,
) -> Result<Json<Job>> {
    let Extension(db) = db_ext;
    let Path((workspace, job_id)) = path;
    // ...
}
```

## Trait Implementations

Use standard trait implementations to simplify conversions and reduce boilerplate:

```rust
// Implement From/Into for type conversions
impl From<DbJob> for ApiJob {
    fn from(db: DbJob) -> Self {
        ApiJob {
            id: db.id,
            status: db.status.into(),
        }
    }
}

// Use TryFrom for fallible conversions
impl TryFrom<String> for JobKind {
    type Error = Error;
    fn try_from(s: String) -> Result<Self, Self::Error> { ... }
}
```

Apply `derive` macros to reduce boilerplate:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job { ... }
```

## Module Structure

- Use `pub(crate)` instead of `pub` when possible; expose only what needs exposing
- Keep APIs small and expressive; avoid leaking internal types
- Organize code into modules reflecting ownership and domain boundaries

```rust
// Prefer restricted visibility
pub(crate) fn internal_helper() { ... }

// Only pub for external API
pub fn create_job(...) -> Result<Job> { ... }
```

## Code Navigation

Always use rust-analyzer LSP for:
- Go to definition
- Find references
- Type information
- Import resolution

Do not guess at module paths or type definitions.

## JSON Handling

Prefer `Box<serde_json::value::RawValue>` over `serde_json::Value` when:
- Storing JSON in the database (JSONB columns)
- Passing JSON through without modification
- The JSON structure doesn't need inspection

```rust
// Preferred - avoids parsing/serialization overhead
pub struct Job {
    pub id: Uuid,
    pub args: Option<Box<serde_json::value::RawValue>>,
}

// Only use Value when you need to inspect/modify JSON
let value: serde_json::Value = serde_json::from_str(&json)?;
if let Some(field) = value.get("field") {
    // modify or inspect
}
```

## Serde Optimizations

Use serde attributes to optimize serialization:

```rust
#[derive(Serialize, Deserialize)]
pub struct Job {
    #[serde(rename = "jobId")]
    pub id: Uuid,

    #[serde(default)]
    pub priority: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<Uuid>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
```

Prefer borrowing for zero-copy deserialization when lifetimes allow:

```rust
#[derive(Deserialize)]
pub struct JobInput<'a> {
    #[serde(borrow)]
    pub workspace_id: Cow<'a, str>,

    #[serde(borrow)]
    pub script_path: &'a str,
}
```

## SQLx Patterns

**Never use `SELECT *`** - always list columns explicitly. This is critical for backwards compatibility when workers run behind the API server version:

```rust
// Preferred - explicit columns
sqlx::query_as!(
    Job,
    "SELECT id, workspace_id, path, created_at FROM v2_job WHERE id = $1",
    job_id
)

// Avoid - breaks when columns are added
sqlx::query_as!(Job, "SELECT * FROM v2_job WHERE id = $1", job_id)
```

Use batch operations to minimize round trips:

```rust
// Preferred - single query with multiple values
sqlx::query!(
    "INSERT INTO job_logs (job_id, logs) VALUES ($1, $2), ($3, $4)",
    id1, log1, id2, log2
)

// Avoid N+1 queries
for id in ids {
    sqlx::query!("SELECT ... WHERE id = $1", id).fetch_one(db).await?;
}

// Preferred - single query with IN clause
sqlx::query!("SELECT ... WHERE id = ANY($1)", &ids[..]).fetch_all(db).await?
```

Use transactions for multi-step operations and parameterize all queries.

## Async & Tokio Patterns

Never block the async runtime. Use `spawn_blocking` for CPU-intensive or blocking I/O:

```rust
// Preferred - offload blocking work
let result = tokio::task::spawn_blocking(move || {
    expensive_computation(&data)
}).await?;

// Avoid - blocks the runtime
let result = expensive_computation(&data);  // Don't do this in async
```

Use tokio primitives for sleep and channels:

```rust
use tokio::sync::mpsc;
use tokio::time::sleep;

// Avoid in async contexts
use std::thread::sleep; // Blocks the runtime
```

Use bounded channels for backpressure:

```rust
// Preferred - bounded channel prevents overwhelming
let (tx, rx) = tokio::sync::mpsc::channel(100);

// Be careful with unbounded
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
```

## Mutex Selection in Async Code

**Prefer `std::sync::Mutex` (or `parking_lot::Mutex`) over `tokio::sync::Mutex`** for protecting data in async code. The async mutex is more expensive and only needed when holding locks across `.await` points.

```rust
// Preferred for data protection - std mutex is faster
use std::sync::Mutex;

struct Cache {
    data: Mutex<HashMap<String, Value>>,
}

impl Cache {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.lock().unwrap().get(key).cloned()
    }

    fn insert(&self, key: String, value: Value) {
        self.data.lock().unwrap().insert(key, value);
    }
}
```

**Use `tokio::sync::Mutex` only when you must hold the lock across `.await` points**, typically for IO resources like database connections:

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

// Async mutex for IO resources held across await points
let conn = Arc::new(Mutex::new(db_connection));

async fn execute_query(conn: Arc<Mutex<DbConn>>, query: &str) {
    let mut lock = conn.lock().await;
    lock.execute(query).await;  // Lock held across .await
}
```

**Common pattern**: Wrap `Arc<Mutex<...>>` in a struct with non-async methods that lock internally, keeping lock scope minimal:

```rust
struct SharedState {
    inner: std::sync::Mutex<StateInner>,
}

impl SharedState {
    fn update(&self, value: i32) {
        self.inner.lock().unwrap().value = value;
    }

    fn get(&self) -> i32 {
        self.inner.lock().unwrap().value
    }
}
```

**Alternative for IO resources**: Spawn a dedicated task to manage the resource and communicate via message passing:

```rust
let (tx, mut rx) = tokio::sync::mpsc::channel(32);

tokio::spawn(async move {
    while let Some(cmd) = rx.recv().await {
        handle_io_command(&mut resource, cmd).await;
    }
});
```

## Build & Tooling

Build speed tips:
- Use `cargo check` during rapid iteration over `cargo build`
- Minimize unnecessary dependencies and feature flags