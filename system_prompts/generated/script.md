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


# Compiled Languages (C#, Java)

## C#

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

### NuGet Packages

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

## Java

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

### Maven Dependencies

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


# SQL Languages

## PostgreSQL

Arguments are obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc.

Name the parameters by adding comments at the beginning of the script:
```sql
-- $1 name1
-- $2 name2 = default_value
SELECT * FROM users WHERE name = $1::TEXT AND age > $2::INT;
```

## MySQL

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:
```sql
-- ? name1 (text)
-- ? name2 (int) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```

## BigQuery

Arguments use `@name` syntax.

Name the parameters by adding comments before the statement:
```sql
-- @name1 (string)
-- @name2 (int64) = 0
SELECT * FROM users WHERE name = @name1 AND age > @name2;
```

## Snowflake

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:
```sql
-- ? name1 (text)
-- ? name2 (number) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```

## Microsoft SQL Server (MSSQL)

Arguments use `@P1`, `@P2`, etc.

Name the parameters by adding comments before the statement:
```sql
-- @P1 name1 (varchar)
-- @P2 name2 (int) = 0
SELECT * FROM users WHERE name = @P1 AND age > @P2;
```

## DuckDB

Arguments are defined with comments and used with `$name` syntax:
```sql
-- $name (text) = default
-- $age (integer)
SELECT * FROM users WHERE name = $name AND age > $age;
```

### Ducklake Integration
Attach Ducklake for data lake operations:
```sql
-- Main ducklake
ATTACH 'ducklake' AS dl;

-- Named ducklake
ATTACH 'ducklake://my_lake' AS dl;

-- Then query
SELECT * FROM dl.schema.table;
```

### External Database Connections
Connect to external databases using resources:
```sql
ATTACH '$res:path/to/resource' AS db (TYPE postgres);
SELECT * FROM db.schema.table;
```

### S3 File Operations
Read files from S3 storage:
```sql
-- Default storage
SELECT * FROM read_csv('s3:///path/to/file.csv');

-- Named storage
SELECT * FROM read_csv('s3://storage_name/path/to/file.csv');

-- Parquet files
SELECT * FROM read_parquet('s3:///path/to/file.parquet');
```


# TypeScript (Bun/Deno/Native)

## Variants

- `bun`: Bun runtime, full npm ecosystem, fastest execution
- `deno`: Deno runtime, npm via `npm:{package}`, deno libraries supported
- `nativets`, `bunnative`: Native TypeScript, fetch only, no external imports allowed

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

### Bun Runtime
```typescript
import Stripe from 'stripe'
import { someFunction } from 'some-package'
```

### Deno Runtime
```typescript
import Stripe from 'npm:stripe'
import { someFunction } from 'npm:some-package'
// Or deno libraries:
import { serve } from 'https://deno.land/std/http/server.ts'
```

### Native TypeScript (nativets, bunnative)
No imports allowed. Use the globally available `fetch` function:
```typescript
export async function main(url: string) {
  const response = await fetch(url);
  return await response.json();
}
```

## Windmill Client

Import the windmill client for platform interactions:

```typescript
import * as wmill from 'windmill-client'
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
  body: any; // Raw body of the request
  headers: Record<string, string>;
  query: Record<string, string>;
};

export async function preprocessor(event: Event) {
  // Transform the event into flow input parameters
  return {
    param1: event.body.field1,
    param2: event.query.id
  };
}
```


# S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations.

## S3Object Type

The S3Object type represents a file in S3 storage:

```typescript
type S3Object = {
  s3: string;  // Path within the bucket
}
```

## TypeScript Operations

```typescript
import * as wmill from 'windmill-client'

// Load file content from S3
const content: Uint8Array = await wmill.loadS3File(s3object)

// Load file as stream
const blob: Blob = await wmill.loadS3FileStream(s3object)

// Write file to S3
const result: S3Object = await wmill.writeS3File(
  s3object,      // Target path (or undefined to auto-generate)
  fileContent,   // string or Blob
  s3ResourcePath // Optional: specific S3 resource to use
)
```

## Python Operations

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

## DuckDB S3 Operations

Read files directly from S3 in DuckDB queries:

```sql
-- Read CSV from default S3 storage
SELECT * FROM read_csv('s3:///path/to/file.csv');

-- Read from named storage
SELECT * FROM read_csv('s3://storage_name/path/to/file.csv');

-- Read Parquet files
SELECT * FROM read_parquet('s3:///data/*.parquet');

-- Read JSON files
SELECT * FROM read_json('s3:///path/to/file.json');
```

## Flow Input Schema

To accept an S3 object as flow input:

```json
{
  "type": "object",
  "properties": {
    "file": {
      "type": "object",
      "format": "resource-s3_object",
      "description": "File to process"
    }
  }
}
```


# TypeScript SDK (windmill-client)

Import: import * as wmill from 'windmill-client'

getWorkspace(): string - Create a client configuration from env variables
async getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any> - Get a resource value by path
async getRootJobId(jobId?: string): Promise<string> - Get the true root job id
async runScript(path: string | null = null, hash_: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any> - @deprecated Use runScriptByPath or runScriptByHash instead
appendToResultStream(text: string): void - Append a text to the result stream
async streamResult(stream: AsyncIterable<string>): Promise<void> - Stream to the result stream
async runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null): Promise<string> - @deprecated Use runScriptByPathAsync or runScriptByHashAsync instead
async resolveDefaultResource(obj: any): Promise<any> - Resolve a resource value in case the default value was picked because the input payload was undefined
async setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void> - Set a resource value by path
async setInternalState(state: any): Promise<void> - Set the state
async setState(state: any): Promise<void> - Set the state
async setProgress(percent: number, jobId?: any): Promise<void> - Set the progress
async getProgress(jobId?: any): Promise<number | null> - Get the progress
async setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void> - Set a flow user state
async getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any> - Get a flow user state
async getInternalState(): Promise<any> - Get the internal state
async getState(): Promise<any> - Get the state shared across executions
async getVariable(path: string): Promise<string> - Get a variable by path
async setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void> - Set a variable by path, create if not exist
async writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath: string | undefined = undefined, contentType: string | undefined = undefined, contentDisposition: string | undefined = undefined): Promise<S3Object> - Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
async signS3Objects(s3objects: S3Object[]): Promise<S3Object[]> - Sign S3 objects to be used by anonymous users in public apps
async signS3Object(s3object: S3Object): Promise<S3Object> - Sign S3 object to be used by anonymous users in public apps
async getPresignedS3PublicUrls(s3Objects: S3Object[], { baseUrl }: { baseUrl?: string } = {}): Promise<string[]> - Generate a presigned public URL for an array of S3 objects.
async getPresignedS3PublicUrl(s3Objects: S3Object, { baseUrl }: { baseUrl?: string } = {}): Promise<string> - Generate a presigned public URL for an S3 object. If the S3 object is not signed yet, it will be signed first.
async getResumeUrls(approver?: string): Promise< - Get URLs needed for resuming a flow after this step
getResumeEndpoints(approver?: string): Promise< - @deprecated use getResumeUrls instead
async getIdToken(audience: string, expiresIn?: number): Promise<string> - Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
async usernameToEmail(username: string): Promise<string> - Get email from workspace username
datatable(name: string = "main"): SqlTemplateFunction - @example
ducklake(name: string = "main"): SqlTemplateFunction - @example
setClient(token?: string, baseUrl?: string): void
async runScriptByPath(path: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async runScriptByHash(hash_: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async runFlow(path: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async waitJob(jobId: string, verbose: boolean = false): Promise<any>
async getResult(jobId: string): Promise<any>
async getResultMaybe(jobId: string): Promise<any>
async runScriptByPathAsync(path: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>
async runScriptByHashAsync(hash_: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>
async runFlowAsync(path: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, // can only be set to false if this the job will be fully await and not concurrent with any other job // as otherwise the child flow and its own child will store their state in the parent job which will // lead to incorrectness and failures doNotTrackInParent: boolean = true): Promise<string>
getStatePath(): string
async setSharedState(// state: any, // path = "state.json" //): Promise<void>
async getSharedState(path = "state.json"): Promise<any>
async databaseUrlFromResource(path: string): Promise<string>
async polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any>
async duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any>
async denoS3LightClientSettings(s3_resource_path: string | undefined): Promise<DenoS3LightClientSettings>
async loadS3File(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Uint8Array | undefined>
async loadS3FileStream(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Blob | undefined>
base64ToUint8Array(data: string): Uint8Array
uint8ArrayToBase64(arrayBuffer: Uint8Array): string
async requestInteractiveSlackApproval({ slackResourcePath, channelId, message, approver, defaultArgsJson, dynamicEnumsJson, }: SlackApprovalOptions): Promise<void>
async requestInteractiveTeamsApproval({ teamName, channelName, message, approver, defaultArgsJson, dynamicEnumsJson, }: TeamsApprovalOptions): Promise<void>
parseS3Object(s3Object: S3Object): S3ObjectRecord


# Python SDK (wmill)

Import: import wmill

def init_global_client(f)
def deprecate(in_favor_of: str)
def get_workspace() -> str
def get_root_job_id(job_id: str | None = None) -> str
def get_version() -> str
def run_script_async(hash_or_path: str, args: Dict[str, Any] = None, scheduled_in_secs: int = None) -> str
def run_flow_async(path: str, args: Dict[str, Any] = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str
def run_script_sync(hash: str, args: Dict[str, Any] = None, verbose: bool = False, assert_result_is_not_none: bool = True, cleanup: bool = True, timeout: dt.timedelta = None) -> Any
def run_script_by_path_async(path: str, args: Dict[str, Any] = None, scheduled_in_secs: Union[None, int] = None) -> str
def run_script_by_hash_async(hash_: str, args: Dict[str, Any] = None, scheduled_in_secs: Union[None, int] = None) -> str
def run_script_by_path_sync(path: str, args: Dict[str, Any] = None, verbose: bool = False, assert_result_is_not_none: bool = True, cleanup: bool = True, timeout: dt.timedelta = None) -> Any
def get_id_token(audience: str) -> str - Get a JWT token for the given audience for OIDC purposes to login into third parties like AWS, Vault, GCP, etc.
def get_job_status(job_id: str) -> JobStatus
def get_result(job_id: str, assert_result_is_not_none = True) -> Dict[str, Any]
def duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings - Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings - Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings - Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None = None) -> bytes - Load the entire content of a file stored in S3 as bytes
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None = None) -> BufferedReader - Load the content of a file stored in S3
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None = None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object - Upload a file to S3
def sign_s3_objects(s3_objects: list[S3Object | str]) -> list[S3Object] - Sign S3 objects to be used by anonymous users in public apps
def sign_s3_object(s3_object: S3Object | str) -> S3Object - Sign S3 object to be used by anonymous users in public apps
def get_presigned_s3_public_urls(s3_objects: list[S3Object | str], base_url: str | None = None) -> list[str] - Generate presigned public URLs for an array of S3 objects.
def get_presigned_s3_public_url(s3_object: S3Object | str, base_url: str | None = None) -> str - Generate a presigned public URL for an S3 object.
def whoami() -> dict - Returns the current user
def get_state() -> Any - Get the state
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None - Get resource from Windmill
def set_resource(path: str, value: Any, resource_type: str = 'any') -> None - Set the resource at a given path as a string, creating it if it does not exist
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict] - List resources from Windmill workspace.
def set_state(value: Any) -> None - Set the state
def set_progress(value: int, job_id: Optional[str] = None) -> None - Set the progress
def get_progress(job_id: Optional[str] = None) -> Any - Get the progress
def set_shared_state_pickle(value: Any, path = 'state.pickle') -> None - Set the state in the shared folder using pickle
def get_shared_state_pickle(path = 'state.pickle') -> Any - Get the state in the shared folder using pickle
def set_shared_state(value: Any, path = 'state.json') -> None - Set the state in the shared folder using pickle
def get_shared_state(path = 'state.json') -> None - Get the state in the shared folder using pickle
def get_variable(path: str) -> str - Returns the variable at a given path as a string
def set_variable(path: str, value: str, is_secret: bool = False) -> None - Set the variable at a given path as a string, creating it if it does not exist
def get_flow_user_state(key: str) -> Any - Get the user state of a flow at a given key
def set_flow_user_state(key: str, value: Any) -> None - Set the user state of a flow at a given key
def get_state_path() -> str
def get_resume_urls(approver: str = None) -> dict
def request_interactive_slack_approval(slack_resource_path: str, channel_id: str, message: str = None, approver: str = None, default_args_json: dict = None, dynamic_enums_json: dict = None) -> None
def send_teams_message(conversation_id: str, text: str, success: bool, card_block: dict = None)
def cancel_job(job_id: str, reason: str = None) -> str - Cancel a specific job by ID.
def cancel_running() -> dict - Cancel currently running executions of the same script.
def run_script(path: str = None, hash_: str = None, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any - Run script synchronously and return its result.
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any - Run script by path synchronously and return its result.
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any - Run script by hash synchronously and return its result.
def run_inline_script_preview(content: str, language: str, args: dict = None) -> Any - Run a script on the current worker without creating a job
def username_to_email(username: str) -> str - Get email from workspace username
def datatable(name: str = 'main') -> DataTableClient
def ducklake(name: str = 'main') -> DucklakeClient
def task(*args, **kwargs)
def parse_resource_syntax(s: str) -> Optional[str] - Parse resource syntax from string.
def parse_s3_object(s3_object: S3Object | str) -> S3Object - Parse S3 object from string or S3Object format.
def parse_variable_syntax(s: str) -> Optional[str] - Parse variable syntax from string.
def append_to_result_stream(text: str) -> None - Append a text to the result stream.
def stream_result(stream) -> None - Stream to the result stream.
def infer_sql_type(value) -> str - DuckDB executor requires explicit argument types at declaration
def get_mocked_api() -> Optional[dict]
def get_client() -> httpx.Client
def get(endpoint, raise_for_status = True, **kwargs) -> httpx.Response
def post(endpoint, raise_for_status = True, **kwargs) -> httpx.Response
def create_token(duration = dt.timedelta(days=1)) -> str
def run_script_async(path: str = None, hash_: str = None, args: dict = None, scheduled_in_secs: int = None) -> str - Create a script job and return its job id.
def run_script_by_path_async(path: str, args: dict = None, scheduled_in_secs: int = None) -> str - Create a script job by path and return its job id.
def run_script_by_hash_async(hash_: str, args: dict = None, scheduled_in_secs: int = None) -> str - Create a script job by hash and return its job id.
def run_flow_async(path: str, args: dict = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str - Create a flow job and return its job id.
def run_script(path: str = None, hash_: str = None, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any - Run script synchronously and return its result.
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any - Run script by path synchronously and return its result.
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any - Run script by hash synchronously and return its result.
def run_inline_script_preview(content: str, language: str, args: dict = None) -> Any - Run a script on the current worker without creating a job
def wait_job(job_id, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False)
def cancel_job(job_id: str, reason: str = None) -> str - Cancel a specific job by ID.
def cancel_running() -> dict - Cancel currently running executions of the same script.
def get_job(job_id: str) -> dict
def get_root_job_id(job_id: str | None = None) -> dict
def get_id_token(audience: str, expires_in: int | None = None) -> str
def get_job_status(job_id: str) -> JobStatus
def get_result(job_id: str, assert_result_is_not_none: bool = True) -> Any
def get_variable(path: str) -> str
def set_variable(path: str, value: str, is_secret: bool = False) -> None
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None
def set_resource(value: Any, path: str, resource_type: str)
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict] - List resources from Windmill workspace.
def set_state(value: Any)
def set_progress(value: int, job_id: Optional[str] = None)
def get_progress(job_id: Optional[str] = None) -> Any
def set_flow_user_state(key: str, value: Any) -> None - Set the user state of a flow at a given key
def get_flow_user_state(key: str) -> Any - Get the user state of a flow at a given key
def version()
def get_duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings | None - Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def get_polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings - Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def get_boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings - Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None) -> bytes - Load a file from the workspace s3 bucket and returns its content as bytes.
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None) -> BufferedReader - Load a file from the workspace s3 bucket and returns the bytes stream.
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object - Write a file to the workspace S3 bucket
def sign_s3_objects(s3_objects: list[S3Object | str]) -> list[S3Object]
def sign_s3_object(s3_object: S3Object | str) -> S3Object
def get_presigned_s3_public_urls(s3_objects: list[S3Object | str], base_url: str | None = None) -> list[str] - Generate presigned public URLs for an array of S3 objects.
def get_presigned_s3_public_url(s3_object: S3Object | str, base_url: str | None = None) -> str - Generate a presigned public URL for an S3 object.
def whoami() -> dict
def user() -> dict
def state_path() -> str
def state() -> Any
def state(value: Any) -> None
def set_shared_state_pickle(value: Any, path: str = 'state.pickle') -> None - Set the state in the shared folder using pickle
def get_shared_state_pickle(path: str = 'state.pickle') -> Any - Get the state in the shared folder using pickle
def set_shared_state(value: Any, path: str = 'state.json') -> None - Set the state in the shared folder using pickle
def get_shared_state(path: str = 'state.json') -> None - Get the state in the shared folder using pickle
def get_resume_urls(approver: str = None) -> dict
def request_interactive_slack_approval(slack_resource_path: str, channel_id: str, message: str = None, approver: str = None, default_args_json: dict = None, dynamic_enums_json: dict = None) -> None - Sends an interactive approval request via Slack, allowing optional customization of the message, approver, and form fields.
def username_to_email(username: str) -> str - Get email from workspace username
def send_teams_message(conversation_id: str, text: str, success: bool = True, card_block: dict = None) - Send a message to a Microsoft Teams conversation with conversation_id, where success is used to style the message
def datatable(name: str = 'main')
def ducklake(name: str = 'main')
def wrapper(*args, **kwargs)
def decorator(f)
def f(func, tag: str | None = None)
def query(sql: str, *args)
def query(sql: str, **kwargs)
def fetch(result_collection: str | None = None)
def fetch_one()
def cancel_job()
def wrapper(*args, **kwargs)
def inner(*args, **kwargs)
def inner(*args, **kwargs)
