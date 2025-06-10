# Windmill Rust SDK

A Rust client library for interacting with [Windmill](https://www.windmill.dev/) API, providing type-safe abstractions for variables, resources, scripts, jobs, and state management.

## Installation

Add this to your ```Cargo.toml```:

```toml
[dependencies]
wmill = "0.1.0"
```

For async support (recommended):

```toml
[dependencies]
wmill = { version = "0.1.0", features = ["async"] }
```

## Usage

### Initialize Client

```rust
use wmill::Windmill;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read config from env vars
    let wm = Windmill::default()?;
    
    // Or override specific values
    let wm = Windmill::new(
        Some("custom_token".to_string()),
        Some("my_workspace".to_string()),
        Some("http://localhost:8000".to_string())
    )?;
    
    Ok(())
}
```

### Variables

```rust
// Get variable (auto-parsed)
let db_config: serde_json::Value = wm.get_variable("u/admin/db_config").await?;

// Get raw variable
let raw_text = wm.get_variable_raw("u/user/text_note").await?;

// Set variable
wm.set_variable("new_value".to_string(), "u/user/my_var", false).await?;
```

### Resources

```rust
// Get resource (typed)
#[derive(serde::Deserialize)]
struct DbConfig { host: String, port: u16 }

let config: DbConfig = wm.get_resource("u/admin/db").await?;

// Get raw resource
let raw_json = wm.get_resource_any("u/admin/db").await?;

// Set resource
wm.set_resource(
    Some(serde_json::json!({"host": "localhost", "port": 5432})),
    "u/admin/db",
    "postgresql"
).await?;
```

### Scripts

```rust
// Run script async
let job_id = wm.run_script_async(
    "u/user/my_script",
    false,
    serde_json::json!({"param": "value"}),
    Some(10) // Schedule in 10 seconds
).await?;

// Run script sync
let result = wm.run_script_sync(
    "u/user/my_script",
    false,
    serde_json::json!({"param": "value"}),
    Some(10),
    Some(30), // 30s timeout
    true,     // Verbose
    true      // Assert result not None
).await?;
```

### Jobs

```rust
// Wait for job completion
let result = wm.wait_job(&job_id, Some(60), true, true).await?;

// Get job status
let status = wm.get_job_status(&job_id).await?; // Running/Waiting/Completed

// Get result directly
let result = wm.get_result(&job_id).await?;
```

### State Management

```rust
// Get typed state
#[derive(serde::Deserialize)]
struct ScriptState { counter: i32 }

let state: ScriptState = wm.get_state().await?;

// Get raw state
let raw_state = wm.get_state_any().await?;

// Update state
wm.set_state(Some(serde_json::json!({"counter": 42}))).await?;
```

### Progress Tracking

```rust
// Set job progress
wm.set_progress(75, None).await?; // Uses current job ID from env

// Get job progress
let progress = wm.get_progress(Some(job_id.to_string())).await?;
```

### Custom API Calls

The SDK provides direct access to underlying API endpoints through the ```call_api``` method, which works in both async and sync contexts:

```rust
// Async usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wm = Windmill::default()?;
    
    // Make direct API call to get user
    let user = wm.call_api(wmill::apis::admin_api::get_user(
        &wm.client_config,
        &wm.workspace,
        "Alice"
    )).await;
    
    println!("User details: {:?}", user);
    Ok(())
}

// Sync usage
fn main() {
    let wm = Windmill::default().unwrap();
    
    // Make direct API call to get user
    let user = wm.call_api(wmill::apis::admin_api::get_user(
        &wm.client_config,
        &wm.workspace,
        "Bob"
    ));
    
    println!("User details: {:?}", user);
}
```

This advanced feature allows access to any Windmill API endpoint, even those not covered by the SDK's convenience methods. Use this when:
- Need to access newer/undocumented API endpoints
- Require fine-grained control over API requests
- Existing abstractions don't meet specific needs

## Environment Variables

The SDK uses these environment variables:

| Variable | Required | Description |
|---------|----------|-------------|
| ```WM_TOKEN``` | ✅ | Authentication token |
| ```WM_WORKSPACE``` | ✅ | Workspace name |
| ```BASE_INTERNAL_URL``` | ✅ | API base URL (without ```/api```) |
| ```WM_JOB_ID``` | Optional | Current job ID for progress tracking |
| ```WM_STATE_PATH_NEW``` | Optional | State path override |

## Contributions

Contributions are welcome! Please open an issue or submit a PR.

