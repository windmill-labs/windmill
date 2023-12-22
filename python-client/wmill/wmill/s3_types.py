class S3Object(dict):
    s3: str

    def __getattr__(self, attr):
        return self[attr]


class S3FsClientKwargs(dict):
    region_name: str

    def __getattr__(self, attr):
        return self[attr]


class S3FsArgs(dict):
    endpoint_url: str
    key: str
    secret: str
    use_ssl: bool
    cache_regions: bool
    client_kwargs: S3FsClientKwargs

    def __getattr__(self, attr):
        return self[attr]


class StorageOptions(dict):
    aws_endpoint_url: str
    aws_access_key_id: str
    aws_secret_access_key: str
    aws_region: str
    aws_allow_http: str

    def __getattr__(self, attr):
        return self[attr]


class PolarsConnectionSettings(dict):
    s3fs_args: S3FsArgs
    storage_options: StorageOptions

    def __getattr__(self, attr):
        return self[attr]


class Boto3ConnectionSettings(dict):
    endpoint_url: str
    region_name: str
    use_ssl: bool
    aws_access_key_id: str
    aws_secret_access_key: str

    def __getattr__(self, attr):
        return self[attr]


class DuckDbConnectionSettings(dict):
    connection_settings_str: str

    def __getattr__(self, attr):
        return self[attr]
