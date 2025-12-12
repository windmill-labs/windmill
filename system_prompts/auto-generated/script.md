# Windmill Script Writing Guide

## General Principles

- Scripts must export a main function (do not call it)
- Libraries are installed automatically - do not show installation instructions
- Credentials and configuration are stored in resources and passed as parameters
- The windmill client (`wmill`) provides APIs for interacting with the platform

## Function Naming

- Main function: `main` (or `preprocessor` for preprocessor scripts)
- Must be async for TypeScript variants

## Return Values

- Scripts can return any JSON-serializable value
- Return values become available to subsequent flow steps via `results.step_id`

## Preprocessor Scripts

Preprocessor scripts process raw trigger data from various sources (webhook, custom HTTP route, SQS, WebSocket, Kafka, NATS, MQTT, Postgres, or email) before passing it to the flow. This separates the trigger logic from the flow logic and keeps the auto-generated UI clean.

The returned object determines the parameter values passed to the flow.
e.g., `{ b: 1, a: 2 }` calls the flow with `a = 2` and `b = 1`, assuming the flow has two inputs called `a` and `b`.

The preprocessor receives a single parameter called `event`.


# Bash

## Structure

Do not include `#!/bin/bash`. Arguments are obtained as positional parameters:

```bash
# Get arguments
var1="$1"
var2="$2"

echo "Processing $var1 and $var2"

# Return JSON by echoing to stdout
echo "{\"result\": \"$var1\", \"count\": $var2}"
```

**Important:**
- Do not include shebang (`#!/bin/bash`)
- Arguments are always strings
- Access with `$1`, `$2`, etc.

## Output

The script output is captured as the result. For structured data, output valid JSON:

```bash
name="$1"
count="$2"

# Output JSON result
cat << EOF
{
  "name": "$name",
  "count": $count,
  "timestamp": "$(date -Iseconds)"
}
EOF
```

## Environment Variables

Environment variables set in Windmill are available:

```bash
# Access environment variable
echo "Workspace: $WM_WORKSPACE"
echo "Job ID: $WM_JOB_ID"
```


# BigQuery

Arguments use `@name` syntax.

Name the parameters by adding comments before the statement:

```sql
-- @name1 (string)
-- @name2 (int64) = 0
SELECT * FROM users WHERE name = @name1 AND age > @name2;
```


# TypeScript (Bun)

Bun runtime with full npm ecosystem and fastest execution.

## Structure

Export a single **async** function called `main`:

```typescript
export async function main(param1: string, param2: number) {
  // Your code here
  return { result: param1, count: param2 };
}
```

Do not call the main function. Libraries are installed automatically.

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

Use the `RT` namespace for resource types:

```typescript
export async function main(stripe: RT.Stripe) {
  // stripe contains API key and config from the resource
}
```

Only use resource types if you need them to satisfy the instructions. Always use the RT namespace.

## Imports

```typescript
import Stripe from "stripe";
import { someFunction } from "some-package";
```

## Windmill Client

Import the windmill client for platform interactions:

```typescript
import * as wmill from "windmill-client";
```

See the SDK documentation for available methods.

## Preprocessor Scripts

For preprocessor scripts, the function should be named `preprocessor` and receives an `event` parameter:

```typescript
type Event = {
  kind:
    | "webhook"
    | "http"
    | "websocket"
    | "kafka"
    | "email"
    | "nats"
    | "postgres"
    | "sqs"
    | "mqtt"
    | "gcp";
  body: any;
  headers: Record<string, string>;
  query: Record<string, string>;
};

export async function preprocessor(event: Event) {
  return {
    param1: event.body.field1,
    param2: event.query.id,
  };
}
```

## S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations.

### S3Object Type

The S3Object type represents a file in S3 storage:

```typescript
type S3Object = {
  s3: string; // Path within the bucket
};
```

## TypeScript Operations

```typescript
import * as wmill from "windmill-client";

// Load file content from S3
const content: Uint8Array = await wmill.loadS3File(s3object);

// Load file as stream
const blob: Blob = await wmill.loadS3FileStream(s3object);

// Write file to S3
const result: S3Object = await wmill.writeS3File(
  s3object, // Target path (or undefined to auto-generate)
  fileContent, // string or Blob
  s3ResourcePath // Optional: specific S3 resource to use
);
```


# TypeScript (Bun Native)

Native TypeScript execution with fetch only - no external imports allowed.

## Structure

Export a single **async** function called `main`:

```typescript
export async function main(param1: string, param2: number) {
  // Your code here
  return { result: param1, count: param2 };
}
```

Do not call the main function.

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

Use the `RT` namespace for resource types:

```typescript
export async function main(stripe: RT.Stripe) {
  // stripe contains API key and config from the resource
}
```

Only use resource types if you need them to satisfy the instructions. Always use the RT namespace.

## Imports

**No imports allowed.** Use the globally available `fetch` function:

```typescript
export async function main(url: string) {
  const response = await fetch(url);
  return await response.json();
}
```

## Windmill Client

The windmill client is not available in native TypeScript mode. Use fetch to call APIs directly.

## Preprocessor Scripts

For preprocessor scripts, the function should be named `preprocessor` and receives an `event` parameter:

```typescript
type Event = {
  kind:
    | "webhook"
    | "http"
    | "websocket"
    | "kafka"
    | "email"
    | "nats"
    | "postgres"
    | "sqs"
    | "mqtt"
    | "gcp";
  body: any;
  headers: Record<string, string>;
  query: Record<string, string>;
};

export async function preprocessor(event: Event) {
  return {
    param1: event.body.field1,
    param2: event.query.id,
  };
}
```

## S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations.

### S3Object Type

The S3Object type represents a file in S3 storage:

```typescript
type S3Object = {
  s3: string; // Path within the bucket
};
```

## TypeScript Operations

```typescript
import * as wmill from "windmill-client";

// Load file content from S3
const content: Uint8Array = await wmill.loadS3File(s3object);

// Load file as stream
const blob: Blob = await wmill.loadS3FileStream(s3object);

// Write file to S3
const result: S3Object = await wmill.writeS3File(
  s3object, // Target path (or undefined to auto-generate)
  fileContent, // string or Blob
  s3ResourcePath // Optional: specific S3 resource to use
);
```


# C#

The script must contain a public static `Main` method inside a class:

```csharp
public class Script
{
    public static object Main(string name, int count)
    {
        return new { Name = name, Count = count };
    }
}
```

**Important:**
- Class name is irrelevant
- Method must be `public static`
- Return type can be `object` or specific type

## NuGet Packages

Add packages using the `#r` directive at the top:

```csharp
#r "nuget: Newtonsoft.Json, 13.0.3"
#r "nuget: RestSharp, 110.2.0"

using Newtonsoft.Json;
using RestSharp;

public class Script
{
    public static object Main(string url)
    {
        var client = new RestClient(url);
        var request = new RestRequest();
        var response = client.Get(request);
        return JsonConvert.DeserializeObject(response.Content);
    }
}
```


# TypeScript (Deno)

Deno runtime with npm support via `npm:` prefix and native Deno libraries.

## Structure

Export a single **async** function called `main`:

```typescript
export async function main(param1: string, param2: number) {
  // Your code here
  return { result: param1, count: param2 };
}
```

Do not call the main function. Libraries are installed automatically.

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

Use the `RT` namespace for resource types:

```typescript
export async function main(stripe: RT.Stripe) {
  // stripe contains API key and config from the resource
}
```

Only use resource types if you need them to satisfy the instructions. Always use the RT namespace.

## Imports

```typescript
// npm packages use npm: prefix
import Stripe from "npm:stripe";
import { someFunction } from "npm:some-package";

// Deno standard library
import { serve } from "https://deno.land/std/http/server.ts";
```

## Windmill Client

Import the windmill client for platform interactions:

```typescript
import * as wmill from "windmill-client";
```

See the SDK documentation for available methods.

## Preprocessor Scripts

For preprocessor scripts, the function should be named `preprocessor` and receives an `event` parameter:

```typescript
type Event = {
  kind:
    | "webhook"
    | "http"
    | "websocket"
    | "kafka"
    | "email"
    | "nats"
    | "postgres"
    | "sqs"
    | "mqtt"
    | "gcp";
  body: any;
  headers: Record<string, string>;
  query: Record<string, string>;
};

export async function preprocessor(event: Event) {
  return {
    param1: event.body.field1,
    param2: event.query.id,
  };
}
```

## S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations.

### S3Object Type

The S3Object type represents a file in S3 storage:

```typescript
type S3Object = {
  s3: string; // Path within the bucket
};
```

## TypeScript Operations

```typescript
import * as wmill from "windmill-client";

// Load file content from S3
const content: Uint8Array = await wmill.loadS3File(s3object);

// Load file as stream
const blob: Blob = await wmill.loadS3FileStream(s3object);

// Write file to S3
const result: S3Object = await wmill.writeS3File(
  s3object, // Target path (or undefined to auto-generate)
  fileContent, // string or Blob
  s3ResourcePath // Optional: specific S3 resource to use
);
```


# DuckDB

Arguments are defined with comments and used with `$name` syntax:

```sql
-- $name (text) = default
-- $age (integer)
SELECT * FROM users WHERE name = $name AND age > $age;
```

## Ducklake Integration

Attach Ducklake for data lake operations:

```sql
-- Main ducklake
ATTACH 'ducklake' AS dl;

-- Named ducklake
ATTACH 'ducklake://my_lake' AS dl;

-- Then query
SELECT * FROM dl.schema.table;
```

## External Database Connections

Connect to external databases using resources:

```sql
ATTACH '$res:path/to/resource' AS db (TYPE postgres);
SELECT * FROM db.schema.table;
```

## S3 File Operations

Read files from S3 storage:

```sql
-- Default storage
SELECT * FROM read_csv('s3:///path/to/file.csv');

-- Named storage
SELECT * FROM read_csv('s3://storage_name/path/to/file.csv');

-- Parquet files
SELECT * FROM read_parquet('s3:///path/to/file.parquet');

-- JSON files
SELECT * FROM read_json('s3:///path/to/file.json');
```


# Go

## Structure

The file package must be `inner` and export a function called `main`:

```go
package inner

func main(param1 string, param2 int) (map[string]interface{}, error) {
    return map[string]interface{}{
        "result": param1,
        "count":  param2,
    }, nil
}
```

**Important:**
- Package must be `inner`
- Return type must be `({return_type}, error)`
- Function name is `main` (lowercase)

## Return Types

The return type can be any Go type that can be serialized to JSON:

```go
package inner

type Result struct {
    Name  string `json:"name"`
    Count int    `json:"count"`
}

func main(name string, count int) (Result, error) {
    return Result{
        Name:  name,
        Count: count,
    }, nil
}
```

## Error Handling

Return errors as the second return value:

```go
package inner

import "errors"

func main(value int) (string, error) {
    if value < 0 {
        return "", errors.New("value must be positive")
    }
    return "success", nil
}
```


# GraphQL

## Structure

Write GraphQL queries or mutations. Arguments can be added as query parameters:

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    id
    name
    email
  }
}
```

## Variables

Variables are passed as script arguments and automatically bound to the query:

```graphql
query SearchProducts($query: String!, $limit: Int = 10) {
  products(search: $query, first: $limit) {
    edges {
      node {
        id
        name
        price
      }
    }
  }
}
```

## Mutations

```graphql
mutation CreateUser($input: CreateUserInput!) {
  createUser(input: $input) {
    id
    name
    createdAt
  }
}
```


# Java

The script must contain a Main public class with a `public static main()` method:

```java
public class Main {
    public static Object main(String name, int count) {
        java.util.Map<String, Object> result = new java.util.HashMap<>();
        result.put("name", name);
        result.put("count", count);
        return result;
    }
}
```

**Important:**
- Class must be named `Main`
- Method must be `public static Object main(...)`
- Return type is `Object` or `void`

## Maven Dependencies

Add dependencies using comments at the top:

```java
//requirements:
//com.google.code.gson:gson:2.10.1
//org.apache.httpcomponents:httpclient:4.5.14

import com.google.gson.Gson;

public class Main {
    public static Object main(String input) {
        Gson gson = new Gson();
        return gson.fromJson(input, Object.class);
    }
}
```


# Microsoft SQL Server (MSSQL)

Arguments use `@P1`, `@P2`, etc.

Name the parameters by adding comments before the statement:

```sql
-- @P1 name1 (varchar)
-- @P2 name2 (int) = 0
SELECT * FROM users WHERE name = @P1 AND age > @P2;
```


# MySQL

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:

```sql
-- ? name1 (text)
-- ? name2 (int) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```


# TypeScript (Native)

Native TypeScript execution with fetch only - no external imports allowed.

## Structure

Export a single **async** function called `main`:

```typescript
export async function main(param1: string, param2: number) {
  // Your code here
  return { result: param1, count: param2 };
}
```

Do not call the main function.

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

Use the `RT` namespace for resource types:

```typescript
export async function main(stripe: RT.Stripe) {
  // stripe contains API key and config from the resource
}
```

Only use resource types if you need them to satisfy the instructions. Always use the RT namespace.

## Imports

**No imports allowed.** Use the globally available `fetch` function:

```typescript
export async function main(url: string) {
  const response = await fetch(url);
  return await response.json();
}
```

## Windmill Client

The windmill client is not available in native TypeScript mode. Use fetch to call APIs directly.

## Preprocessor Scripts

For preprocessor scripts, the function should be named `preprocessor` and receives an `event` parameter:

```typescript
type Event = {
  kind:
    | "webhook"
    | "http"
    | "websocket"
    | "kafka"
    | "email"
    | "nats"
    | "postgres"
    | "sqs"
    | "mqtt"
    | "gcp";
  body: any;
  headers: Record<string, string>;
  query: Record<string, string>;
};

export async function preprocessor(event: Event) {
  return {
    param1: event.body.field1,
    param2: event.query.id
  };
}
```


# PHP

## Structure

The script must start with `<?php` and contain at least one function called `main`:

```php
<?php

function main(string $param1, int $param2) {
    return ["result" => $param1, "count" => $param2];
}
```

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

You need to **redefine** the type of the resources that are needed before the main function. Always check if the class already exists using `class_exists`:

```php
<?php

if (!class_exists('Postgresql')) {
    class Postgresql {
        public string $host;
        public int $port;
        public string $user;
        public string $password;
        public string $dbname;
    }
}

function main(Postgresql $db) {
    // $db contains the database connection details
}
```

The resource type name has to be exactly as specified.

## Library Dependencies

Specify library dependencies as comments before the main function:

```php
<?php

// require:
// guzzlehttp/guzzle
// stripe/stripe-php@^10.0

function main() {
    // Libraries are available
}
```

One dependency per line. No need to require autoload, it is already done.


# PostgreSQL

Arguments are obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc.

Name the parameters by adding comments at the beginning of the script (without specifying the type):

```sql
-- $1 name1
-- $2 name2 = default_value
SELECT * FROM users WHERE name = $1::TEXT AND age > $2::INT;
```


# PowerShell

## Structure

Arguments are obtained by calling the `param` function on the first line:

```powershell
param($Name, $Count = 0, [int]$Age)

# Your code here
Write-Output "Processing $Name, count: $Count, age: $Age"

# Return object
@{
    name = $Name
    count = $Count
    age = $Age
}
```

## Parameter Types

You can specify types for parameters:

```powershell
param(
    [string]$Name,
    [int]$Count = 0,
    [bool]$Enabled = $true,
    [array]$Items
)

@{
    name = $Name
    count = $Count
    enabled = $Enabled
    items = $Items
}
```

## Return Values

Return values by outputting them at the end of the script:

```powershell
param($Input)

$result = @{
    processed = $true
    data = $Input
    timestamp = Get-Date -Format "o"
}

$result
```


# Python

## Structure

The script must contain at least one function called `main`:

```python
def main(param1: str, param2: int):
    # Your code here
    return {"result": param1, "count": param2}
```

Do not call the main function. Libraries are installed automatically.

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

You need to **redefine** the type of the resources that are needed before the main function as TypedDict:

```python
from typing import TypedDict

class postgresql(TypedDict):
    host: str
    port: int
    user: str
    password: str
    dbname: str

def main(db: postgresql):
    # db contains the database connection details
    pass
```

**Important rules:**

- The resource type name must be **IN LOWERCASE**
- Only include resource types if they are actually needed
- If an import conflicts with a resource type name, **rename the imported object, not the type name**
- Make sure to import TypedDict from typing **if you're using it**

## Imports

Libraries are installed automatically. Do not show installation instructions.

```python
import requests
import pandas as pd
from datetime import datetime
```

If an import name conflicts with a resource type:

```python
# Wrong - don't rename the type
import stripe as stripe_lib
class stripe_type(TypedDict): ...

# Correct - rename the import
import stripe as stripe_sdk
class stripe(TypedDict):
    api_key: str
```

## Windmill Client

Import the windmill client for platform interactions:

```python
import wmill
```

See the SDK documentation for available methods.

## Preprocessor Scripts

For preprocessor scripts, the function should be named `preprocessor` and receives an `event` parameter:

```python
from typing import TypedDict, Literal, Any

class Event(TypedDict):
    kind: Literal["webhook", "http", "websocket", "kafka", "email", "nats", "postgres", "sqs", "mqtt", "gcp"]
    body: Any
    headers: dict[str, str]
    query: dict[str, str]

def preprocessor(event: Event):
    # Transform the event into flow input parameters
    return {
        "param1": event["body"]["field1"],
        "param2": event["query"]["id"]
    }
```

## S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations.

```python
import wmill

# Load file content from S3
content: bytes = wmill.load_s3_file(s3object)

# Load file as stream reader
reader: BufferedReader = wmill.load_s3_file_reader(s3object)

# Write file to S3
result: S3Object = wmill.write_s3_file(
    s3object,           # Target path (or None to auto-generate)
    file_content,       # bytes or BufferedReader
    s3_resource_path,   # Optional: specific S3 resource
    content_type,       # Optional: MIME type
    content_disposition # Optional: Content-Disposition header
)
```


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


# Snowflake

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:

```sql
-- ? name1 (text)
-- ? name2 (number) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```


# TypeScript SDK (windmill-client)

Import: import * as wmill from 'windmill-client'

/**
 * Initialize the Windmill client with authentication token and base URL
 * @param token - Authentication token (defaults to WM_TOKEN env variable)
 * @param baseUrl - API base URL (defaults to BASE_INTERNAL_URL or BASE_URL env variable)
 */
setClient(token?: string, baseUrl?: string): void

/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
getWorkspace(): string

/**
 * Get a resource value by path
 * @param path path of the resource,  default to internal state path
 * @param undefinedIfEmpty if the resource does not exist, return undefined instead of throwing an error
 * @returns resource value
 */
async getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any>

/**
 * Get the true root job id
 * @param jobId job id to get the root job id from (default to current job)
 * @returns root job id
 */
async getRootJobId(jobId?: string): Promise<string>

/**
 * @deprecated Use runScriptByPath or runScriptByHash instead
 */
async runScript(path: string | null = null, hash_: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

/**
 * Run a script synchronously by its path and wait for the result
 * @param path - Script path in Windmill
 * @param args - Arguments to pass to the script
 * @param verbose - Enable verbose logging
 * @returns Script execution result
 */
async runScriptByPath(path: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

/**
 * Run a script synchronously by its hash and wait for the result
 * @param hash_ - Script hash in Windmill
 * @param args - Arguments to pass to the script
 * @param verbose - Enable verbose logging
 * @returns Script execution result
 */
async runScriptByHash(hash_: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

/**
 * Append a text to the result stream
 * @param text text to append to the result stream
 */
appendToResultStream(text: string): void

/**
 * Stream to the result stream
 * @param stream stream to stream to the result stream
 */
async streamResult(stream: AsyncIterable<string>): Promise<void>

/**
 * Run a flow synchronously by its path and wait for the result
 * @param path - Flow path in Windmill
 * @param args - Arguments to pass to the flow
 * @param verbose - Enable verbose logging
 * @returns Flow execution result
 */
async runFlow(path: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

/**
 * Wait for a job to complete and return its result
 * @param jobId - ID of the job to wait for
 * @param verbose - Enable verbose logging
 * @returns Job result when completed
 */
async waitJob(jobId: string, verbose: boolean = false): Promise<any>

/**
 * Get the result of a completed job
 * @param jobId - ID of the completed job
 * @returns Job result
 */
async getResult(jobId: string): Promise<any>

/**
 * Get the result of a job if completed, or its current status
 * @param jobId - ID of the job
 * @returns Object with started, completed, success, and result properties
 */
async getResultMaybe(jobId: string): Promise<any>

/**
 * Wrap a function to execute as a Windmill task within a flow context
 * @param f - Function to wrap as a task
 * @returns Async wrapper function that executes as a Windmill job
 */
task<P, T>(f: (_: P) => T): (_: P) => Promise<T>

/**
 * @deprecated Use runScriptByPathAsync or runScriptByHashAsync instead
 */
async runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null): Promise<string>

/**
 * Run a script asynchronously by its path
 * @param path - Script path in Windmill
 * @param args - Arguments to pass to the script
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @returns Job ID of the created job
 */
async runScriptByPathAsync(path: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>

/**
 * Run a script asynchronously by its hash
 * @param hash_ - Script hash in Windmill
 * @param args - Arguments to pass to the script
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @returns Job ID of the created job
 */
async runScriptByHashAsync(hash_: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>

/**
 * Run a flow asynchronously by its path
 * @param path - Flow path in Windmill
 * @param args - Arguments to pass to the flow
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @param doNotTrackInParent - If false, tracks state in parent job (only use when fully awaiting the job)
 * @returns Job ID of the created job
 */
async runFlowAsync(path: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, // can only be set to false if this the job will be fully await and not concurrent with any other job // as otherwise the child flow and its own child will store their state in the parent job which will // lead to incorrectness and failures doNotTrackInParent: boolean = true): Promise<string>

/**
 * Resolve a resource value in case the default value was picked because the input payload was undefined
 * @param obj resource value or path of the resource under the format `$res:path`
 * @returns resource value
 */
async resolveDefaultResource(obj: any): Promise<any>

/**
 * Get the state file path from environment variables
 * @returns State path string
 */
getStatePath(): string

/**
 * Set a resource value by path
 * @param path path of the resource to set, default to state path
 * @param value new value of the resource to set
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
async setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void>

/**
 * Set the state
 * @param state state to set
 * @deprecated use setState instead
 */
async setInternalState(state: any): Promise<void>

/**
 * Set the state
 * @param state state to set
 */
async setState(state: any): Promise<void>

/**
 * Set the progress
 * Progress cannot go back and limited to 0% to 99% range
 * @param percent Progress to set in %
 * @param jobId? Job to set progress for
 */
async setProgress(percent: number, jobId?: any): Promise<void>

/**
 * Get the progress
 * @param jobId? Job to get progress from
 * @returns Optional clamped between 0 and 100 progress value
 */
async getProgress(jobId?: any): Promise<number | null>

/**
 * Set a flow user state
 * @param key key of the state
 * @param value value of the state
 */
async setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void>

/**
 * Get a flow user state
 * @param path path of the variable
 */
async getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any>

/**
 * Get the internal state
 * @deprecated use getState instead
 */
async getInternalState(): Promise<any>

/**
 * Get the state shared across executions
 */
async getState(): Promise<any>

/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
async getVariable(path: string): Promise<string>

/**
 * Set a variable by path, create if not exist
 * @param path path of the variable
 * @param value value of the variable
 * @param isSecretIfNotExist if the variable does not exist, create it as secret or not (default: false)
 * @param descriptionIfNotExist if the variable does not exist, create it with this description (default: "")
 */
async setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void>

/**
 * Build a PostgreSQL connection URL from a database resource
 * @param path - Path to the database resource
 * @returns PostgreSQL connection URL string
 */
async databaseUrlFromResource(path: string): Promise<string>

/**
 * Get S3 client settings from a resource or workspace default
 * @param s3_resource_path - Path to S3 resource (uses workspace default if undefined)
 * @returns S3 client configuration settings
 */
async denoS3LightClientSettings(s3_resource_path: string | undefined): Promise<DenoS3LightClientSettings>

/**
 * Load the content of a file stored in S3. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 * 
 * ```typescript
 * let fileContent = await wmill.loadS3FileContent(inputFile)
 * // if the file is a raw text file, it can be decoded and printed directly:
 * const text = new TextDecoder().decode(fileContentStream)
 * console.log(text);
 * ```
 */
async loadS3File(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Uint8Array | undefined>

/**
 * Load the content of a file stored in S3 as a stream. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 * 
 * ```typescript
 * let fileContentBlob = await wmill.loadS3FileStream(inputFile)
 * // if the content is plain text, the blob can be read directly:
 * console.log(await fileContentBlob.text());
 * ```
 */
async loadS3FileStream(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Blob | undefined>

/**
 * Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 * 
 * ```typescript
 * const s3object = await writeS3File(s3Object, "Hello Windmill!")
 * const fileContentAsUtf8Str = (await s3object.toArray()).toString('utf-8')
 * console.log(fileContentAsUtf8Str)
 * ```
 */
async writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath: string | undefined = undefined, contentType: string | undefined = undefined, contentDisposition: string | undefined = undefined): Promise<S3Object>

/**
 * Sign S3 objects to be used by anonymous users in public apps
 * @param s3objects s3 objects to sign
 * @returns signed s3 objects
 */
async signS3Objects(s3objects: S3Object[]): Promise<S3Object[]>

/**
 * Sign S3 object to be used by anonymous users in public apps
 * @param s3object s3 object to sign
 * @returns signed s3 object
 */
async signS3Object(s3object: S3Object): Promise<S3Object>

/**
 * Generate a presigned public URL for an array of S3 objects.
 * If an S3 object is not signed yet, it will be signed first.
 * @param s3Objects s3 objects to sign
 * @returns list of signed public URLs
 */
async getPresignedS3PublicUrls(s3Objects: S3Object[], { baseUrl }: { baseUrl?: string } = {}): Promise<string[]>

/**
 * Generate a presigned public URL for an S3 object. If the S3 object is not signed yet, it will be signed first.
 * @param s3Object s3 object to sign
 * @returns signed public URL
 */
async getPresignedS3PublicUrl(s3Objects: S3Object, { baseUrl }: { baseUrl?: string } = {}): Promise<string>

/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @returns approval page UI URL, resume and cancel API URLs for resuming the flow
 */
async getResumeUrls(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}>

/**
 * @deprecated use getResumeUrls instead
 */
getResumeEndpoints(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}>

/**
 * Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
 * @param audience audience of the token
 * @param expiresIn Optional number of seconds until the token expires
 * @returns jwt token
 */
async getIdToken(audience: string, expiresIn?: number): Promise<string>

/**
 * Convert a base64-encoded string to Uint8Array
 * @param data - Base64-encoded string
 * @returns Decoded Uint8Array
 */
base64ToUint8Array(data: string): Uint8Array

/**
 * Convert a Uint8Array to base64-encoded string
 * @param arrayBuffer - Uint8Array to encode
 * @returns Base64-encoded string
 */
uint8ArrayToBase64(arrayBuffer: Uint8Array): string

/**
 * Get email from workspace username
 * This method is particularly useful for apps that require the email address of the viewer.
 * Indeed, in the viewer context, WM_USERNAME is set to the username of the viewer but WM_EMAIL is set to the email of the creator of the app.
 * @param username
 * @returns email address
 */
async usernameToEmail(username: string): Promise<string>

/**
 * Sends an interactive approval request via Slack, allowing optional customization of the message, approver, and form fields.
 * 
 * **[Enterprise Edition Only]** To include form fields in the Slack approval request, go to **Advanced -> Suspend -> Form**
 * and define a form. Learn more at [Windmill Documentation](https://www.windmill.dev/docs/flows/flow_approval#form).
 * 
 * @param {Object} options - The configuration options for the Slack approval request.
 * @param {string} options.slackResourcePath - The path to the Slack resource in Windmill.
 * @param {string} options.channelId - The Slack channel ID where the approval request will be sent.
 * @param {string} [options.message] - Optional custom message to include in the Slack approval request.
 * @param {string} [options.approver] - Optional user ID or name of the approver for the request.
 * @param {DefaultArgs} [options.defaultArgsJson] - Optional object defining or overriding the default arguments to a form field.
 * @param {Enums} [options.dynamicEnumsJson] - Optional object overriding the enum default values of an enum form field.
 * 
 * @returns {Promise<void>} Resolves when the Slack approval request is successfully sent.
 * 
 * @throws {Error} If the function is not called within a flow or flow preview.
 * @throws {Error} If the `JobService.getSlackApprovalPayload` call fails.
 * 
 * **Usage Example:**
 * ```typescript
 * await requestInteractiveSlackApproval({
 *   slackResourcePath: "/u/alex/my_slack_resource",
 *   channelId: "admins-slack-channel",
 *   message: "Please approve this request",
 *   approver: "approver123",
 *   defaultArgsJson: { key1: "value1", key2: 42 },
 *   dynamicEnumsJson: { foo: ["choice1", "choice2"], bar: ["optionA", "optionB"] },
 * });
 * ```
 * 
 * **Note:** This function requires execution within a Windmill flow or flow preview.
 */
async requestInteractiveSlackApproval({ slackResourcePath, channelId, message, approver, defaultArgsJson, dynamicEnumsJson, }: SlackApprovalOptions): Promise<void>

/**
 * Sends an interactive approval request via Teams, allowing optional customization of the message, approver, and form fields.
 * 
 * **[Enterprise Edition Only]** To include form fields in the Teams approval request, go to **Advanced -> Suspend -> Form**
 * and define a form. Learn more at [Windmill Documentation](https://www.windmill.dev/docs/flows/flow_approval#form).
 * 
 * @param {Object} options - The configuration options for the Teams approval request.
 * @param {string} options.teamName - The Teams team name where the approval request will be sent.
 * @param {string} options.channelName - The Teams channel name where the approval request will be sent.
 * @param {string} [options.message] - Optional custom message to include in the Teams approval request.
 * @param {string} [options.approver] - Optional user ID or name of the approver for the request.
 * @param {DefaultArgs} [options.defaultArgsJson] - Optional object defining or overriding the default arguments to a form field.
 * @param {Enums} [options.dynamicEnumsJson] - Optional object overriding the enum default values of an enum form field.
 * 
 * @returns {Promise<void>} Resolves when the Teams approval request is successfully sent.
 * 
 * @throws {Error} If the function is not called within a flow or flow preview.
 * @throws {Error} If the `JobService.getTeamsApprovalPayload` call fails.
 * 
 * **Usage Example:**
 * ```typescript
 * await requestInteractiveTeamsApproval({
 *   teamName: "admins-teams",
 *   channelName: "admins-teams-channel",
 *   message: "Please approve this request",
 *   approver: "approver123",
 *   defaultArgsJson: { key1: "value1", key2: 42 },
 *   dynamicEnumsJson: { foo: ["choice1", "choice2"], bar: ["optionA", "optionB"] },
 * });
 * ```
 * 
 * **Note:** This function requires execution within a Windmill flow or flow preview.
 */
async requestInteractiveTeamsApproval({ teamName, channelName, message, approver, defaultArgsJson, dynamicEnumsJson, }: TeamsApprovalOptions): Promise<void>

/**
 * Parse an S3 object from URI string or record format
 * @param s3Object - S3 object as URI string (s3://storage/key) or record
 * @returns S3 object record with storage and s3 key
 */
parseS3Object(s3Object: S3Object): S3ObjectRecord

/**
 * Create a SQL template function for PostgreSQL/datatable queries
 * @param name - Database/datatable name (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.datatable()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}::int
 * `.fetch()
 */
datatable(name: string = "main"): SqlTemplateFunction

/**
 * Create a SQL template function for DuckDB/ducklake queries
 * @param name - DuckDB database name (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.ducklake()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}
 * `.fetch()
 */
ducklake(name: string = "main"): SqlTemplateFunction

async polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any>

async duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any>


# Python SDK (wmill)

Import: import wmill

def get_mocked_api() -> Optional[dict]

# Get the HTTP client instance.
# 
# Returns:
#     Configured httpx.Client for API requests
def get_client() -> httpx.Client

# Make an HTTP GET request to the Windmill API.
# 
# Args:
#     endpoint: API endpoint path
#     raise_for_status: Whether to raise an exception on HTTP errors
#     **kwargs: Additional arguments passed to httpx.get
# 
# Returns:
#     HTTP response object
def get(endpoint, raise_for_status = True, **kwargs) -> httpx.Response

# Make an HTTP POST request to the Windmill API.
# 
# Args:
#     endpoint: API endpoint path
#     raise_for_status: Whether to raise an exception on HTTP errors
#     **kwargs: Additional arguments passed to httpx.post
# 
# Returns:
#     HTTP response object
def post(endpoint, raise_for_status = True, **kwargs) -> httpx.Response

# Create a new authentication token.
# 
# Args:
#     duration: Token validity duration (default: 1 day)
# 
# Returns:
#     New authentication token string
def create_token(duration = dt.timedelta(days=1)) -> str

# Create a script job and return its job id.
# 
# .. deprecated:: Use run_script_by_path_async or run_script_by_hash_async instead.
def run_script_async(path: str = None, hash_: str = None, args: dict = None, scheduled_in_secs: int = None) -> str

# Create a script job by path and return its job id.
def run_script_by_path_async(path: str, args: dict = None, scheduled_in_secs: int = None) -> str

# Create a script job by hash and return its job id.
def run_script_by_hash_async(hash_: str, args: dict = None, scheduled_in_secs: int = None) -> str

# Create a flow job and return its job id.
def run_flow_async(path: str, args: dict = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str

# Run script synchronously and return its result.
# 
# .. deprecated:: Use run_script_by_path or run_script_by_hash instead.
def run_script(path: str = None, hash_: str = None, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any

# Run script by path synchronously and return its result.
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any

# Run script by hash synchronously and return its result.
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any

# Run a script on the current worker without creating a job
def run_inline_script_preview(content: str, language: str, args: dict = None) -> Any

# Wait for a job to complete and return its result.
# 
# Args:
#     job_id: ID of the job to wait for
#     timeout: Maximum time to wait (seconds or timedelta)
#     verbose: Enable verbose logging
#     cleanup: Register cleanup handler to cancel job on exit
#     assert_result_is_not_none: Raise exception if result is None
# 
# Returns:
#     Job result when completed
# 
# Raises:
#     TimeoutError: If timeout is reached
#     Exception: If job fails
def wait_job(job_id, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False)

# Cancel a specific job by ID.
# 
# Args:
#     job_id: UUID of the job to cancel
#     reason: Optional reason for cancellation
# 
# Returns:
#     Response message from the cancel endpoint
def cancel_job(job_id: str, reason: str = None) -> str

# Cancel currently running executions of the same script.
def cancel_running() -> dict

# Get job details by ID.
# 
# Args:
#     job_id: UUID of the job
# 
# Returns:
#     Job details dictionary
def get_job(job_id: str) -> dict

# Get the root job ID for a flow hierarchy.
# 
# Args:
#     job_id: Job ID (defaults to current WM_JOB_ID)
# 
# Returns:
#     Root job ID
def get_root_job_id(job_id: str | None = None) -> dict

# Get an OIDC JWT token for authentication to external services.
# 
# Args:
#     audience: Token audience (e.g., "vault", "aws")
#     expires_in: Optional expiration time in seconds
# 
# Returns:
#     JWT token string
def get_id_token(audience: str, expires_in: int | None = None) -> str

# Get the status of a job.
# 
# Args:
#     job_id: UUID of the job
# 
# Returns:
#     Job status: "RUNNING", "WAITING", or "COMPLETED"
def get_job_status(job_id: str) -> JobStatus

# Get the result of a completed job.
# 
# Args:
#     job_id: UUID of the completed job
#     assert_result_is_not_none: Raise exception if result is None
# 
# Returns:
#     Job result
def get_result(job_id: str, assert_result_is_not_none: bool = True) -> Any

# Get a variable value by path.
# 
# Args:
#     path: Variable path in Windmill
# 
# Returns:
#     Variable value as string
def get_variable(path: str) -> str

# Set a variable value by path, creating it if it doesn't exist.
# 
# Args:
#     path: Variable path in Windmill
#     value: Variable value to set
#     is_secret: Whether the variable should be secret (default: False)
def set_variable(path: str, value: str, is_secret: bool = False) -> None

# Get a resource value by path.
# 
# Args:
#     path: Resource path in Windmill
#     none_if_undefined: Return None instead of raising if not found
# 
# Returns:
#     Resource value dictionary or None
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None

# Set a resource value by path, creating it if it doesn't exist.
# 
# Args:
#     value: Resource value to set
#     path: Resource path in Windmill
#     resource_type: Resource type for creation
def set_resource(value: Any, path: str, resource_type: str)

# List resources from Windmill workspace.
# 
# Args:
#     resource_type: Optional resource type to filter by (e.g., "postgresql", "mysql", "s3")
#     page: Optional page number for pagination
#     per_page: Optional number of results per page
#     
# Returns:
#     List of resource dictionaries
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict]

# Set the workflow state.
# 
# Args:
#     value: State value to set
def set_state(value: Any)

# Set job progress percentage (0-99).
# 
# Args:
#     value: Progress percentage
#     job_id: Job ID (defaults to current WM_JOB_ID)
def set_progress(value: int, job_id: Optional[str] = None)

# Get job progress percentage.
# 
# Args:
#     job_id: Job ID (defaults to current WM_JOB_ID)
# 
# Returns:
#     Progress value (0-100) or None if not set
def get_progress(job_id: Optional[str] = None) -> Any

# Set the user state of a flow at a given key
def set_flow_user_state(key: str, value: Any) -> None

# Get the user state of a flow at a given key
def get_flow_user_state(key: str) -> Any

# Get the Windmill server version.
# 
# Returns:
#     Version string
def version()

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
# initiate an S3 connection from DuckDB
def get_duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings | None

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
# initiate an S3 connection from Polars
def get_polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
# initiate an S3 connection using boto3
def get_boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings

# Load a file from the workspace s3 bucket and returns its content as bytes.
# 
# '''python
# from wmill import S3Object
# 
# s3_obj = S3Object(s3="/path/to/my_file.txt")
# my_obj_content = client.load_s3_file(s3_obj)
# file_content = my_obj_content.decode("utf-8")
# '''
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None) -> bytes

# Load a file from the workspace s3 bucket and returns the bytes stream.
# 
# '''python
# from wmill import S3Object
# 
# s3_obj = S3Object(s3="/path/to/my_file.txt")
# with wmill.load_s3_file_reader(s3object, s3_resource_path) as file_reader:
#     print(file_reader.read())
# '''
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None) -> BufferedReader

# Write a file to the workspace S3 bucket
# 
# '''python
# from wmill import S3Object
# 
# s3_obj = S3Object(s3="/path/to/my_file.txt")
# 
# # for an in memory bytes array:
# file_content = b'Hello Windmill!'
# client.write_s3_file(s3_obj, file_content)
# 
# # for a file:
# with open("my_file.txt", "rb") as my_file:
#     client.write_s3_file(s3_obj, my_file)
# '''
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object

# Sign S3 objects for use by anonymous users in public apps.
# 
# Args:
#     s3_objects: List of S3 objects to sign
# 
# Returns:
#     List of signed S3 objects
def sign_s3_objects(s3_objects: list[S3Object | str]) -> list[S3Object]

# Sign a single S3 object for use by anonymous users in public apps.
# 
# Args:
#     s3_object: S3 object to sign
# 
# Returns:
#     Signed S3 object
def sign_s3_object(s3_object: S3Object | str) -> S3Object

# Generate presigned public URLs for an array of S3 objects.
# If an S3 object is not signed yet, it will be signed first.
# 
# Args:
#     s3_objects: List of S3 objects to sign
#     base_url: Optional base URL for the presigned URLs (defaults to WM_BASE_URL)
# 
# Returns:
#     List of signed public URLs
# 
# Example:
#     >>> s3_objs = [S3Object(s3="/path/to/file1.txt"), S3Object(s3="/path/to/file2.txt")]
#     >>> urls = client.get_presigned_s3_public_urls(s3_objs)
def get_presigned_s3_public_urls(s3_objects: list[S3Object | str], base_url: str | None = None) -> list[str]

# Generate a presigned public URL for an S3 object.
# If the S3 object is not signed yet, it will be signed first.
# 
# Args:
#     s3_object: S3 object to sign
#     base_url: Optional base URL for the presigned URL (defaults to WM_BASE_URL)
# 
# Returns:
#     Signed public URL
# 
# Example:
#     >>> s3_obj = S3Object(s3="/path/to/file.txt")
#     >>> url = client.get_presigned_s3_public_url(s3_obj)
def get_presigned_s3_public_url(s3_object: S3Object | str, base_url: str | None = None) -> str

# Get the current user information.
# 
# Returns:
#     User details dictionary
def whoami() -> dict

# Get the current user information (alias for whoami).
# 
# Returns:
#     User details dictionary
def user() -> dict

# Get the state resource path from environment.
# 
# Returns:
#     State path string
def state_path() -> str

# Get the workflow state.
# 
# Returns:
#     State value or None if not set
def state() -> Any

# Set the state in the shared folder using pickle
def set_shared_state_pickle(value: Any, path: str = 'state.pickle') -> None

# Get the state in the shared folder using pickle
def get_shared_state_pickle(path: str = 'state.pickle') -> Any

# Set the state in the shared folder using pickle
def set_shared_state(value: Any, path: str = 'state.json') -> None

# Get the state in the shared folder using pickle
def get_shared_state(path: str = 'state.json') -> None

# Get URLs needed for resuming a flow after suspension.
# 
# Args:
#     approver: Optional approver name
# 
# Returns:
#     Dictionary with approvalPage, resume, and cancel URLs
def get_resume_urls(approver: str = None) -> dict

# Sends an interactive approval request via Slack, allowing optional customization of the message, approver, and form fields.
# 
# **[Enterprise Edition Only]** To include form fields in the Slack approval request, use the "Advanced -> Suspend -> Form" functionality.
# Learn more at: https://www.windmill.dev/docs/flows/flow_approval#form
# 
# :param slack_resource_path: The path to the Slack resource in Windmill.
# :type slack_resource_path: str
# :param channel_id: The Slack channel ID where the approval request will be sent.
# :type channel_id: str
# :param message: Optional custom message to include in the Slack approval request.
# :type message: str, optional
# :param approver: Optional user ID or name of the approver for the request.
# :type approver: str, optional
# :param default_args_json: Optional dictionary defining or overriding the default arguments for form fields.
# :type default_args_json: dict, optional
# :param dynamic_enums_json: Optional dictionary overriding the enum default values of enum form fields.
# :type dynamic_enums_json: dict, optional
# 
# :raises Exception: If the function is not called within a flow or flow preview.
# :raises Exception: If the required flow job or flow step environment variables are not set.
# 
# :return: None
# 
# **Usage Example:**
#     >>> client.request_interactive_slack_approval(
#     ...     slack_resource_path="/u/alex/my_slack_resource",
#     ...     channel_id="admins-slack-channel",
#     ...     message="Please approve this request",
#     ...     approver="approver123",
#     ...     default_args_json={"key1": "value1", "key2": 42},
#     ...     dynamic_enums_json={"foo": ["choice1", "choice2"], "bar": ["optionA", "optionB"]},
#     ... )
# 
# **Notes:**
# - This function must be executed within a Windmill flow or flow preview.
# - The function checks for required environment variables (`WM_FLOW_JOB_ID`, `WM_FLOW_STEP_ID`) to ensure it is run in the appropriate context.
def request_interactive_slack_approval(slack_resource_path: str, channel_id: str, message: str = None, approver: str = None, default_args_json: dict = None, dynamic_enums_json: dict = None) -> None

# Get email from workspace username
# This method is particularly useful for apps that require the email address of the viewer.
# Indeed, in the viewer context WM_USERNAME is set to the username of the viewer but WM_EMAIL is set to the email of the creator of the app.
def username_to_email(username: str) -> str

# Send a message to a Microsoft Teams conversation with conversation_id, where success is used to style the message
def send_teams_message(conversation_id: str, text: str, success: bool = True, card_block: dict = None)

# Get a DataTable client for SQL queries.
# 
# Args:
#     name: Database name (default: "main")
# 
# Returns:
#     DataTableClient instance
def datatable(name: str = 'main')

# Get a DuckLake client for DuckDB queries.
# 
# Args:
#     name: Database name (default: "main")
# 
# Returns:
#     DucklakeClient instance
def ducklake(name: str = 'main')

def init_global_client(f)

def deprecate(in_favor_of: str)

# Get the current workspace ID.
# 
# Returns:
#     Workspace ID string
def get_workspace() -> str

def get_version() -> str

# Run a script synchronously by hash and return its result.
# 
# Args:
#     hash: Script hash
#     args: Script arguments
#     verbose: Enable verbose logging
#     assert_result_is_not_none: Raise exception if result is None
#     cleanup: Register cleanup handler to cancel job on exit
#     timeout: Maximum time to wait
# 
# Returns:
#     Script result
def run_script_sync(hash: str, args: Dict[str, Any] = None, verbose: bool = False, assert_result_is_not_none: bool = True, cleanup: bool = True, timeout: dt.timedelta = None) -> Any

# Run a script synchronously by path and return its result.
# 
# Args:
#     path: Script path
#     args: Script arguments
#     verbose: Enable verbose logging
#     assert_result_is_not_none: Raise exception if result is None
#     cleanup: Register cleanup handler to cancel job on exit
#     timeout: Maximum time to wait
# 
# Returns:
#     Script result
def run_script_by_path_sync(path: str, args: Dict[str, Any] = None, verbose: bool = False, assert_result_is_not_none: bool = True, cleanup: bool = True, timeout: dt.timedelta = None) -> Any

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
# initiate an S3 connection from DuckDB
def duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
# initiate an S3 connection from Polars
def polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
# initiate an S3 connection using boto3
def boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings

# Get the state
def get_state() -> Any

# Get the state resource path from environment.
# 
# Returns:
#     State path string
def get_state_path() -> str

# Decorator to mark a function as a workflow task.
# 
# When executed inside a Windmill job, the decorated function runs as a
# separate workflow step. Outside Windmill, it executes normally.
# 
# Args:
#     tag: Optional worker tag for execution
# 
# Returns:
#     Decorated function
def task(*args, **kwargs)

# Parse resource syntax from string.
def parse_resource_syntax(s: str) -> Optional[str]

# Parse S3 object from string or S3Object format.
def parse_s3_object(s3_object: S3Object | str) -> S3Object

# Parse variable syntax from string.
def parse_variable_syntax(s: str) -> Optional[str]

# Append a text to the result stream.
# 
# Args:
#     text: text to append to the result stream
def append_to_result_stream(text: str) -> None

# Stream to the result stream.
# 
# Args:
#     stream: stream to stream to the result stream
def stream_result(stream) -> None

# Execute a SQL query against the DataTable.
# 
# Args:
#     sql: SQL query string with $1, $2, etc. placeholders
#     *args: Positional arguments to bind to query placeholders
# 
# Returns:
#     SqlQuery instance for fetching results
def query(sql: str, *args)

# Execute query and fetch results.
# 
# Args:
#     result_collection: Optional result collection mode
# 
# Returns:
#     Query results
def fetch(result_collection: str | None = None)

# Execute query and fetch first row of results.
# 
# Returns:
#     First row of query results
def fetch_one()

# DuckDB executor requires explicit argument types at declaration
# These types exist in both DuckDB and Postgres
# Check that the types exist if you plan to extend this function for other SQL engines.
def infer_sql_type(value) -> str

