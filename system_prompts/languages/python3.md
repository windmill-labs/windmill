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
