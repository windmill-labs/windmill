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
def get_id_token(audience: str) -> str
def get_job_status(job_id: str) -> JobStatus
def get_result(job_id: str, assert_result_is_not_none = True) -> Dict[str, Any]
def duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings
def polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings
def boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None = None) -> bytes
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None = None) -> BufferedReader
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None = None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object
def sign_s3_objects(s3_objects: list[S3Object | str]) -> list[S3Object]
def sign_s3_object(s3_object: S3Object | str) -> S3Object
def get_presigned_s3_public_urls(s3_objects: list[S3Object | str], base_url: str | None = None) -> list[str]
def get_presigned_s3_public_url(s3_object: S3Object | str, base_url: str | None = None) -> str
def whoami() -> dict
def get_state() -> Any
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None
def set_resource(path: str, value: Any, resource_type: str = 'any') -> None
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict]
def set_state(value: Any) -> None
def set_progress(value: int, job_id: Optional[str] = None) -> None
def get_progress(job_id: Optional[str] = None) -> Any
def set_shared_state_pickle(value: Any, path = 'state.pickle') -> None
def get_shared_state_pickle(path = 'state.pickle') -> Any
def set_shared_state(value: Any, path = 'state.json') -> None
def get_shared_state(path = 'state.json') -> None
def get_variable(path: str) -> str
def set_variable(path: str, value: str, is_secret: bool = False) -> None
def get_flow_user_state(key: str) -> Any
def set_flow_user_state(key: str, value: Any) -> None
def get_state_path() -> str
def get_resume_urls(approver: str = None) -> dict
def request_interactive_slack_approval(slack_resource_path: str, channel_id: str, message: str = None, approver: str = None, default_args_json: dict = None, dynamic_enums_json: dict = None) -> None
def send_teams_message(conversation_id: str, text: str, success: bool, card_block: dict = None)
def cancel_job(job_id: str, reason: str = None) -> str
def cancel_running() -> dict
def run_script(path: str = None, hash_: str = None, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any
def run_inline_script_preview(content: str, language: str, args: dict = None) -> Any
def username_to_email(username: str) -> str
def datatable(name: str = 'main') -> DataTableClient
def ducklake(name: str = 'main') -> DucklakeClient
def task(*args, **kwargs)
def parse_resource_syntax(s: str) -> Optional[str]
def parse_s3_object(s3_object: S3Object | str) -> S3Object
def parse_variable_syntax(s: str) -> Optional[str]
def append_to_result_stream(text: str) -> None
def stream_result(stream) -> None
def infer_sql_type(value) -> str
def get_mocked_api() -> Optional[dict]
def get_client() -> httpx.Client
def get(endpoint, raise_for_status = True, **kwargs) -> httpx.Response
def post(endpoint, raise_for_status = True, **kwargs) -> httpx.Response
def create_token(duration = dt.timedelta(days=1)) -> str
def run_script_async(path: str = None, hash_: str = None, args: dict = None, scheduled_in_secs: int = None) -> str
def run_script_by_path_async(path: str, args: dict = None, scheduled_in_secs: int = None) -> str
def run_script_by_hash_async(hash_: str, args: dict = None, scheduled_in_secs: int = None) -> str
def run_flow_async(path: str, args: dict = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str
def run_script(path: str = None, hash_: str = None, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any
def run_script_by_path(path: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any
def run_script_by_hash(hash_: str, args: dict = None, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any
def run_inline_script_preview(content: str, language: str, args: dict = None) -> Any
def wait_job(job_id, timeout: dt.timedelta | int | float | None = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False)
def cancel_job(job_id: str, reason: str = None) -> str
def cancel_running() -> dict
def get_job(job_id: str) -> dict
def get_root_job_id(job_id: str | None = None) -> dict
def get_id_token(audience: str, expires_in: int | None = None) -> str
def get_job_status(job_id: str) -> JobStatus
def get_result(job_id: str, assert_result_is_not_none: bool = True) -> Any
def get_variable(path: str) -> str
def set_variable(path: str, value: str, is_secret: bool = False) -> None
def get_resource(path: str, none_if_undefined: bool = False) -> dict | None
def set_resource(value: Any, path: str, resource_type: str)
def list_resources(resource_type: str = None, page: int = None, per_page: int = None) -> list[dict]
def set_state(value: Any)
def set_progress(value: int, job_id: Optional[str] = None)
def get_progress(job_id: Optional[str] = None) -> Any
def set_flow_user_state(key: str, value: Any) -> None
def get_flow_user_state(key: str) -> Any
def version()
def get_duckdb_connection_settings(s3_resource_path: str = '') -> DuckDbConnectionSettings | None
def get_polars_connection_settings(s3_resource_path: str = '') -> PolarsConnectionSettings
def get_boto3_connection_settings(s3_resource_path: str = '') -> Boto3ConnectionSettings
def load_s3_file(s3object: S3Object | str, s3_resource_path: str | None) -> bytes
def load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None) -> BufferedReader
def write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object
def sign_s3_objects(s3_objects: list[S3Object | str]) -> list[S3Object]
def sign_s3_object(s3_object: S3Object | str) -> S3Object
def get_presigned_s3_public_urls(s3_objects: list[S3Object | str], base_url: str | None = None) -> list[str]
def get_presigned_s3_public_url(s3_object: S3Object | str, base_url: str | None = None) -> str
def whoami() -> dict
def user() -> dict
def state_path() -> str
def state() -> Any
def state(value: Any) -> None
def set_shared_state_pickle(value: Any, path: str = 'state.pickle') -> None
def get_shared_state_pickle(path: str = 'state.pickle') -> Any
def set_shared_state(value: Any, path: str = 'state.json') -> None
def get_shared_state(path: str = 'state.json') -> None
def get_resume_urls(approver: str = None) -> dict
def request_interactive_slack_approval(slack_resource_path: str, channel_id: str, message: str = None, approver: str = None, default_args_json: dict = None, dynamic_enums_json: dict = None) -> None
def username_to_email(username: str) -> str
def send_teams_message(conversation_id: str, text: str, success: bool = True, card_block: dict = None)
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
