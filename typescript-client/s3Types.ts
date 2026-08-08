/**
 * S3 object representation, either as a URI string or a record object
 */
export type S3Object = S3ObjectURI | S3ObjectRecord;

/**
 * S3 object URI in the format `s3://storage/key` (`s3:///key` targets the
 * workspace default storage)
 */
export type S3ObjectURI = `s3://${string}/${string}`;

/**
 * S3 object record with file key, optional storage identifier, and optional presigned token
 */
export type S3ObjectRecord = {
  /** File key/path in S3 bucket */
  s3: string;
  /** Storage backend identifier */
  storage?: string;
  /** Presigned URL query string for public access */
  presigned?: string;
};

/**
 * S3 client configuration settings for Deno S3 light client
 */
export type DenoS3LightClientSettings = {
  /** S3 endpoint URL */
  endPoint: string;
  /** AWS region */
  region: string;
  /** Bucket name */
  bucket?: string;
  /** Use HTTPS connection */
  useSSL?: boolean;
  /** AWS access key */
  accessKey?: string;
  /** AWS secret key */
  secretKey?: string;
  /** Use path-style URLs instead of virtual-hosted style */
  pathStyle?: boolean;
};

/**
 * Parse an S3 object from URI string or record format
 * @param s3Object - S3 object as URI string (`s3://storage/key`, `s3:///key`
 *   for the default storage) or record. Any other string throws rather than
 *   falling back to an auto-generated key: an auto key is requested by
 *   omitting the object, and a fallback would silently misplace the upload
 *   on any typo.
 * @returns S3 object record with storage and s3 key
 */
export function parseS3Object(s3Object: S3Object): S3ObjectRecord {
  if (typeof s3Object === "object") return s3Object;
  const match = s3Object.match(/^s3:\/\/([^/]*)\/(.+)$/);
  if (match) return { storage: match[1] || undefined, s3: match[2] };
  if (s3Object.startsWith("s3://")) {
    throw new Error(
      `Invalid s3 object URI ${JSON.stringify(s3Object)}: expected s3://<storage>/<key> with a non-empty key (s3:///<key> for the default storage)`
    );
  }
  throw new Error(
    `Invalid s3 object ${JSON.stringify(s3Object)}: expected an s3://<storage>/<key> URI (e.g. "s3:///${s3Object}" for key "${s3Object}" in the default storage) or { s3: <key> }`
  );
}

/**
 * Settings necessary to connect DuckDB to an S3 bucket
 */
export type DuckDbConnectionSettings = {
  /** DuckDB SET statements to configure the S3 connection */
  connection_settings_str: string;
  /** Azure container path when the target storage is Azure */
  azure_container_path?: string;
};

/**
 * S3 filesystem arguments consumed by Python's Polars `scan_parquet`/`read_parquet`
 */
export type PolarsS3FsArgs = {
  endpoint_url: string;
  key?: string;
  secret?: string;
  use_ssl: boolean;
  cache_regions?: boolean;
  client_kwargs?: Record<string, unknown>;
};

/**
 * Object-storage options consumed by Python's Polars when reading S3 data
 */
export type PolarsStorageOptions = {
  aws_endpoint_url: string;
  aws_region: string;
  aws_allow_http: string;
  aws_access_key_id?: string;
  aws_secret_access_key?: string;
};

/**
 * Settings necessary to connect Polars to an S3 bucket
 */
export type PolarsConnectionSettings = {
  s3fs_args: PolarsS3FsArgs;
  storage_options?: PolarsStorageOptions;
};

/**
 * Settings necessary to connect a boto3 client to an S3 bucket
 */
export type Boto3ConnectionSettings = {
  endpoint_url: string;
  region_name: string;
  use_ssl: boolean;
  aws_access_key_id: string;
  aws_secret_access_key: string;
  aws_session_token?: string;
};
