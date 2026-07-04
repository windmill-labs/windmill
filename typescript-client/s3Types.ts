/**
 * S3 object representation: a bare object key (default storage), a
 * `s3://storage/key` URI string, or a record object
 */
export type S3Object = S3ObjectKey | S3ObjectURI | S3ObjectRecord;

/**
 * Bare object key in the workspace default storage, e.g. `"dir/file.json"`.
 * Equivalent to `{ s3: "dir/file.json" }`.
 */
export type S3ObjectKey = string;

/**
 * S3 object URI in the format `s3://storage/key`
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
 * Parse an S3 object from a bare key, URI string or record format
 * @param s3Object - S3 object as bare key ("dir/file.json", default storage),
 *   URI string (s3://storage/key) or record
 * @returns S3 object record with storage and s3 key
 */
export function parseS3Object(s3Object: S3Object): S3ObjectRecord {
  if (typeof s3Object === "object") return s3Object;
  const match = s3Object.match(/^s3:\/\/([^/]*)\/(.*)$/);
  if (match) return { storage: match[1] || undefined, s3: match[2] ?? "" };
  // A plain string is a bare object key in the default storage. Only a
  // malformed `s3://…` string (no key part) keeps the legacy empty-key
  // fallback — never store a literal "s3://…" key.
  return { s3: s3Object.startsWith("s3://") ? "" : s3Object };
}
