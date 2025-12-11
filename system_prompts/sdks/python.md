# Python SDK (wmill)

Import: import wmill

def init_global_client(f)

def deprecate(in_favor_of: str)

# Get the current workspace ID.
def get_workspace() -> str

# Get the root job ID for a flow hierarchy.
def get_root_job_id(job_id: str | None = None) -> str

def get_version() -> str

# Create a script job and return its job ID.
def run_script_async(hash_or_path: str, args: Dict[str, Any] = None, scheduled_in_secs: int = None) -> str

# Create a flow job and return its job ID.
def run_flow_async(path: str, args: Dict[str, Any] = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str

# Run a script synchronously by hash and return its result.
def run_script_sync(hash: str, args: Dict[str, Any] = None, verbose: bool = False, assert_result_is_not_none: bool = True, cleanup: bool = True, timeout: dt.timedelta = None) -> Any

# Create a script job by path and return its job ID.
def run_script_by_path_async(path: str, args: Dict[str, Any] = None, scheduled_in_secs: Union[None, int] = None) -> str

# Create a script job by hash and return its job ID.
def run_script_by_hash_async(hash_: str, args: Dict[str, Any] = None, scheduled_in_secs: Union[None, int] = None) -> str

# Run a script synchronously by path and return its result.
def run_script_by_path_sync(path: str, args: Dict[str, Any] = None, verbose: bool = False, assert_result_is_not_none: bool = True, cleanup: bool = True, timeout: dt.timedelta = None) -> Any

# Get a JWT token for the given audience for OIDC purposes to login into third parties like AWS, Vault, GCP, etc.
def get_id_token(audience: str) -> str

# Get the status of a job.
def get_job_status(job_id: str) -> JobStatus

# Get the result of a completed job.
def get_result(job_id: str, assert_result_is_not_none = True) -> Dict[str, Any]

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings

# Load the entire content of a file stored in S3 as bytes
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None = None) -> bytes

# Load the content of a file stored in S3
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None = None) -> BufferedReader

# Upload a file to S3
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None = None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object

# Sign S3 objects to be used by anonymous users in public apps
def sign_s3_objects(s3_objects: list[S3Object | str]) -> list[S3Object]

# Sign S3 object to be used by anonymous users in public apps
def sign_s3_object(s3_object: S3Object | str) -> S3Object

# Generate presigned public URLs for an array of S3 objects.
def get_presigned_s3_public_urls(s3_objects: list[S3Object | str], base_url: str | None = None) -> list[str]

# Generate a presigned public URL for an S3 object.
def get_presigned_s3_public_url(s3_object: S3Object | str, base_url: str | None = None) -> str

# Returns the current user
def whoami() -> dict

# Get the state
def get_state() -> Any

# Get resource from Windmill
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None

# Set the resource at a given path as a string, creating it if it does not exist
def set_resource(path: str, value: Any, resource_type: str = 'any') -> None

# List resources from Windmill workspace.
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict]

# Set the state
def set_state(value: Any) -> None

# Set the progress
def set_progress(value: int, job_id: Optional[str] = None) -> None

# Get the progress
def get_progress(job_id: Optional[str] = None) -> Any

# Set the state in the shared folder using pickle
def set_shared_state_pickle(value: Any, path = 'state.pickle') -> None

# Get the state in the shared folder using pickle
def get_shared_state_pickle(path = 'state.pickle') -> Any

# Set the state in the shared folder using pickle
def set_shared_state(value: Any, path = 'state.json') -> None

# Get the state in the shared folder using pickle
def get_shared_state(path = 'state.json') -> None

# Returns the variable at a given path as a string
def get_variable(path: str) -> str

# Set the variable at a given path as a string, creating it if it does not exist
def set_variable(path: str, value: str, is_secret: bool = False) -> None

# Get the user state of a flow at a given key
def get_flow_user_state(key: str) -> Any

# Set the user state of a flow at a given key
def set_flow_user_state(key: str, value: Any) -> None

# Get the state resource path from environment.
def get_state_path() -> str

# Get URLs needed for resuming a flow after suspension.
def get_resume_urls(approver: str = None) -> dict

def request_interactive_slack_approval(slack_resource_path: str, channel_id: str, message: str = None, approver: str = None, default_args_json: dict = None, dynamic_enums_json: dict = None) -> None

# Send a message to a Microsoft Teams conversation.
def send_teams_message(conversation_id: str, text: str, success: bool, card_block: dict = None)

# Cancel a specific job by ID.
def cancel_job(job_id: str, reason: str = None) -> str

# Cancel currently running executions of the same script.
def cancel_running() -> dict

# Run script synchronously and return its result.
def run_script(path: str = None, hash_: str = None, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any

# Run script by path synchronously and return its result.
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any

# Run script by hash synchronously and return its result.
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any

# Run a script on the current worker without creating a job
def run_inline_script_preview(content: str, language: str, args: dict = None) -> Any

# Get email from workspace username
def username_to_email(username: str) -> str

# Get a DataTable client for SQL queries.
def datatable(name: str = 'main') -> DataTableClient

# Get a DuckLake client for DuckDB queries.
def ducklake(name: str = 'main') -> DucklakeClient

# Decorator to mark a function as a workflow task.
def task(*args, **kwargs)

# Parse resource syntax from string.
def parse_resource_syntax(s: str) -> Optional[str]

# Parse S3 object from string or S3Object format.
def parse_s3_object(s3_object: S3Object | str) -> S3Object

# Parse variable syntax from string.
def parse_variable_syntax(s: str) -> Optional[str]

# Append a text to the result stream.
def append_to_result_stream(text: str) -> None

# Stream to the result stream.
def stream_result(stream) -> None

# DuckDB executor requires explicit argument types at declaration
def infer_sql_type(value) -> str

def get_mocked_api() -> Optional[dict]

# Get the HTTP client instance.
def get_client() -> httpx.Client

# Make an HTTP GET request to the Windmill API.
def get(endpoint, raise_for_status = True, **kwargs) -> httpx.Response

# Make an HTTP POST request to the Windmill API.
def post(endpoint, raise_for_status = True, **kwargs) -> httpx.Response

# Create a new authentication token.
def create_token(duration = dt.timedelta(days=1)) -> str

# Create a script job and return its job id.
def run_script_async(path: str = None, hash_: str = None, args: dict = None, scheduled_in_secs: int = None) -> str

# Create a script job by path and return its job id.
def run_script_by_path_async(path: str, args: dict = None, scheduled_in_secs: int = None) -> str

# Create a script job by hash and return its job id.
def run_script_by_hash_async(hash_: str, args: dict = None, scheduled_in_secs: int = None) -> str

# Create a flow job and return its job id.
def run_flow_async(path: str, args: dict = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str

# Run script synchronously and return its result.
def run_script(path: str = None, hash_: str = None, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any

# Run script by path synchronously and return its result.
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any

# Run script by hash synchronously and return its result.
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any

# Run a script on the current worker without creating a job
def run_inline_script_preview(content: str, language: str, args: dict = None) -> Any

# Wait for a job to complete and return its result.
def wait_job(job_id, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False)

# Cancel a specific job by ID.
def cancel_job(job_id: str, reason: str = None) -> str

# Cancel currently running executions of the same script.
def cancel_running() -> dict

# Get job details by ID.
def get_job(job_id: str) -> dict

# Get the root job ID for a flow hierarchy.
def get_root_job_id(job_id: str | None = None) -> dict

# Get an OIDC JWT token for authentication to external services.
def get_id_token(audience: str, expires_in: int | None = None) -> str

# Get the status of a job.
def get_job_status(job_id: str) -> JobStatus

# Get the result of a completed job.
def get_result(job_id: str, assert_result_is_not_none: bool = True) -> Any

# Get a variable value by path.
def get_variable(path: str) -> str

# Set a variable value by path, creating it if it doesn't exist.
def set_variable(path: str, value: str, is_secret: bool = False) -> None

# Get a resource value by path.
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None

# Set a resource value by path, creating it if it doesn't exist.
def set_resource(value: Any, path: str, resource_type: str)

# List resources from Windmill workspace.
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict]

# Set the workflow state.
def set_state(value: Any)

# Set job progress percentage (0-99).
def set_progress(value: int, job_id: Optional[str] = None)

# Get job progress percentage.
def get_progress(job_id: Optional[str] = None) -> Any

# Set the user state of a flow at a given key
def set_flow_user_state(key: str, value: Any) -> None

# Get the user state of a flow at a given key
def get_flow_user_state(key: str) -> Any

# Get the Windmill server version.
def version()

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def get_duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings | None

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def get_polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings

# Convenient helpers that takes an S3 resource as input and returns the settings necessary to
def get_boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings

# Load a file from the workspace s3 bucket and returns its content as bytes.
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None) -> bytes

# Load a file from the workspace s3 bucket and returns the bytes stream.
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None) -> BufferedReader

# Write a file to the workspace S3 bucket
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object

# Sign S3 objects for use by anonymous users in public apps.
def sign_s3_objects(s3_objects: list[S3Object | str]) -> list[S3Object]

# Sign a single S3 object for use by anonymous users in public apps.
def sign_s3_object(s3_object: S3Object | str) -> S3Object

# Generate presigned public URLs for an array of S3 objects.
def get_presigned_s3_public_urls(s3_objects: list[S3Object | str], base_url: str | None = None) -> list[str]

# Generate a presigned public URL for an S3 object.
def get_presigned_s3_public_url(s3_object: S3Object | str, base_url: str | None = None) -> str

# Get the current user information.
def whoami() -> dict

# Get the current user information (alias for whoami).
def user() -> dict

# Get the state resource path from environment.
def state_path() -> str

# Get the workflow state.
def state() -> Any

# Set the workflow state.
def state(value: Any) -> None

# Set the state in the shared folder using pickle
def set_shared_state_pickle(value: Any, path: str = 'state.pickle') -> None

# Get the state in the shared folder using pickle
def get_shared_state_pickle(path: str = 'state.pickle') -> Any

# Set the state in the shared folder using pickle
def set_shared_state(value: Any, path: str = 'state.json') -> None

# Get the state in the shared folder using pickle
def get_shared_state(path: str = 'state.json') -> None

# Get URLs needed for resuming a flow after suspension.
def get_resume_urls(approver: str = None) -> dict

# Sends an interactive approval request via Slack, allowing optional customization of the message, approver, and form fields.
def request_interactive_slack_approval(slack_resource_path: str, channel_id: str, message: str = None, approver: str = None, default_args_json: dict = None, dynamic_enums_json: dict = None) -> None

# Get email from workspace username
def username_to_email(username: str) -> str

# Send a message to a Microsoft Teams conversation with conversation_id, where success is used to style the message
def send_teams_message(conversation_id: str, text: str, success: bool = True, card_block: dict = None)

# Get a DataTable client for SQL queries.
def datatable(name: str = 'main')

# Get a DuckLake client for DuckDB queries.
def ducklake(name: str = 'main')

def wrapper(*args, **kwargs)

def decorator(f)

def f(func, tag: str | None = None)

# Execute a SQL query against the DataTable.
def query(sql: str, *args)

# Execute a DuckDB query against the DuckLake database.
def query(sql: str, **kwargs)

# Execute query and fetch results.
def fetch(result_collection: str | None = None)

# Execute query and fetch first row of results.
def fetch_one()

def cancel_job()

def wrapper(*args, **kwargs)

def inner(*args, **kwargs)

def inner(*args, **kwargs)

