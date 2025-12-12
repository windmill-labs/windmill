from typing import Optional


class S3Object(dict):
    """S3 file reference with file key, optional storage identifier, and presigned token."""
    s3: str
    storage: Optional[str]
    presigned: Optional[str]

    def __getattr__(self, attr):
        return self[attr]


class S3FsClientKwargs(dict):
    """S3FS client keyword arguments for region configuration."""
    region_name: str

    def __getattr__(self, attr):
        return self[attr]


class S3FsArgs(dict):
    """S3FS connection arguments including endpoint, credentials, and client settings."""
    endpoint_url: str
    key: str
    secret: str
    use_ssl: bool
    cache_regions: bool
    client_kwargs: S3FsClientKwargs

    def __getattr__(self, attr):
        return self[attr]


class StorageOptions(dict):
    """Storage options for Polars S3 connectivity with AWS credentials and endpoint."""
    aws_endpoint_url: str
    aws_access_key_id: str
    aws_secret_access_key: str
    aws_region: str
    aws_allow_http: str

    def __getattr__(self, attr):
        return self[attr]


class PolarsConnectionSettings(dict):
    """Polars S3 connection settings containing S3FS args and storage options."""
    s3fs_args: S3FsArgs
    storage_options: StorageOptions

    def __getattr__(self, attr):
        return self[attr]


class Boto3ConnectionSettings(dict):
    """Boto3 S3 connection settings with endpoint, region, and AWS credentials."""
    endpoint_url: str
    region_name: str
    use_ssl: bool
    aws_access_key_id: str
    aws_secret_access_key: str

    def __getattr__(self, attr):
        return self[attr]


class DuckDbConnectionSettings(dict):
    """DuckDB S3 connection settings as a configuration string."""
    connection_settings_str: str

    def __getattr__(self, attr):
        return self[attr]
