# Rust

## Structure

The script must contain a function called `main` with proper return type:

```rust
use anyhow::anyhow;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct ReturnType {
    result: String,
    count: i32,
}

fn main(param1: String, param2: i32) -> anyhow::Result<ReturnType> {
    Ok(ReturnType {
        result: param1,
        count: param2,
    })
}
```

**Important:**
- Arguments should be owned types
- Return type must be serializable (`#[derive(Serialize)]`)
- Return type is `anyhow::Result<T>`

## Dependencies

Packages must be specified with a partial cargo.toml at the beginning of the script:

```rust
//! ```cargo
//! [dependencies]
//! anyhow = "1.0.86"
//! reqwest = { version = "0.11", features = ["json"] }
//! tokio = { version = "1", features = ["full"] }
//! ```

use anyhow::anyhow;
// ... rest of the code
```

**Note:** Serde is already included, no need to add it again.

## Async Functions

If you need to handle async functions (e.g., using tokio), keep the main function sync and create the runtime inside:

```rust
//! ```cargo
//! [dependencies]
//! anyhow = "1.0.86"
//! tokio = { version = "1", features = ["full"] }
//! reqwest = { version = "0.11", features = ["json"] }
//! ```

use anyhow::anyhow;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct Response {
    data: String,
}

fn main(url: String) -> anyhow::Result<Response> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let resp = reqwest::get(&url).await?.text().await?;
        Ok(Response { data: resp })
    })
}
```
