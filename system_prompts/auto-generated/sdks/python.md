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

