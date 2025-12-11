# Python SDK (wmill)

Import: `import wmill`

## Module Functions

These functions use a global client instance automatically initialized from environment variables.

```python
def run_script_async(hash_or_path: str, args: Dict[str, Any] = None, scheduled_in_secs: int = None) -> str
```

```python
def run_flow_async(path: str, args: Dict[str, Any] = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str
```

```python
def get_job_status(job_id: str) -> JobStatus
```

```python
def get_result(job_id: str, assert_result_is_not_none = True) -> Dict[str, Any]
```

```python
def duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings
```
Convenient helpers that takes an S3 resource as input and returns the settings necessary to

```python
def polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings
```
Convenient helpers that takes an S3 resource as input and returns the settings necessary to

```python
def boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings
```
Convenient helpers that takes an S3 resource as input and returns the settings necessary to

```python
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None = None) -> bytes
```
Load the entire content of a file stored in S3 as bytes

```python
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None = None) -> BufferedReader
```
Load the content of a file stored in S3

```python
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None = None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object
```
Upload a file to S3

```python
def whoami() -> dict
```
Returns the current user

```python
def get_state() -> Any
```
Get the state

```python
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None
```
Get resource from Windmill

```python
def set_resource(path: str, value: Any, resource_type: str = 'any') -> None
```
Set the resource at a given path as a string, creating it if it does not exist

```python
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict]
```
List resources from Windmill workspace.

```python
def set_state(value: Any) -> None
```
Set the state

```python
def set_progress(value: int, job_id: Optional[str] = None) -> None
```
Set the progress

```python
def get_progress(job_id: Optional[str] = None) -> Any
```
Get the progress

```python
def get_variable(path: str) -> str
```
Returns the variable at a given path as a string

```python
def set_variable(path: str, value: str, is_secret: bool = False) -> None
```
Set the variable at a given path as a string, creating it if it does not exist

```python
def get_flow_user_state(key: str) -> Any
```
Get the user state of a flow at a given key

```python
def set_flow_user_state(key: str, value: Any) -> None
```
Set the user state of a flow at a given key

```python
def get_resume_urls(approver: str = None) -> dict
```

```python
def cancel_job(job_id: str, reason: str = None) -> str
```
Cancel a specific job by ID.

```python
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any
```
Run script by path synchronously and return its result.

```python
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any
```
Run script by hash synchronously and return its result.

```python
def username_to_email(username: str) -> str
```
Get email from workspace username

```python
def run_script_async(path: str = None, hash_: str = None, args: dict = None, scheduled_in_secs: int = None) -> str
```
Create a script job and return its job id.

```python
def run_flow_async(path: str, args: dict = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str
```
Create a flow job and return its job id.

```python
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any
```
Run script by path synchronously and return its result.

```python
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any
```
Run script by hash synchronously and return its result.

```python
def wait_job(job_id, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False)
```

```python
def cancel_job(job_id: str, reason: str = None) -> str
```
Cancel a specific job by ID.

```python
def get_job_status(job_id: str) -> JobStatus
```

```python
def get_result(job_id: str, assert_result_is_not_none: bool = True) -> Any
```

```python
def get_variable(path: str) -> str
```

```python
def set_variable(path: str, value: str, is_secret: bool = False) -> None
```

```python
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None
```

```python
def set_resource(value: Any, path: str, resource_type: str)
```

```python
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict]
```
List resources from Windmill workspace.

```python
def set_state(value: Any)
```

```python
def set_progress(value: int, job_id: Optional[str] = None)
```

```python
def get_progress(job_id: Optional[str] = None) -> Any
```

```python
def set_flow_user_state(key: str, value: Any) -> None
```
Set the user state of a flow at a given key

```python
def get_flow_user_state(key: str) -> Any
```
Get the user state of a flow at a given key

```python
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None) -> bytes
```
Load a file from the workspace s3 bucket and returns its content as bytes.

```python
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None) -> BufferedReader
```
Load a file from the workspace s3 bucket and returns the bytes stream.

```python
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object
```
Write a file to the workspace S3 bucket

```python
def whoami() -> dict
```

```python
def get_resume_urls(approver: str = None) -> dict
```

```python
def username_to_email(username: str) -> str
```
Get email from workspace username

```python
def cancel_job()
```

## Windmill Class

For more control, instantiate the Windmill class directly:

```python
from wmill import Windmill

client = Windmill()
```

### Key Methods

- `__init__()` - 
- `get_mocked_api()` - 
- `get_client()` - 
- `get()` - 
- `post()` - 
- `create_token()` - 
- `run_script_async()` - Create a script job and return its job id.
- `run_script_by_path_async()` - Create a script job by path and return its job id.
- `run_script_by_hash_async()` - Create a script job by hash and return its job id.
- `run_flow_async()` - Create a flow job and return its job id.
- `run_script()` - Run script synchronously and return its result.
- `run_script_by_path()` - Run script by path synchronously and return its result.
- `run_script_by_hash()` - Run script by hash synchronously and return its result.
- `run_inline_script_preview()` - Run a script on the current worker without creating a job
- `wait_job()` - 
- `cancel_job()` - Cancel a specific job by ID.
- `cancel_running()` - Cancel currently running executions of the same script.
- `get_job()` - 
- `get_root_job_id()` - 
- `get_id_token()` - 

## Types

```python
from wmill import S3Object

# S3Object is a TypedDict with 's3' key for the path
s3obj = S3Object(s3="path/to/file.txt")
```

