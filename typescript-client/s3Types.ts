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
 *   for the default storage) or record. Any other string throws — older
 *   clients silently degraded it to an empty key (the object landed under an
 *   auto-generated key), which hid typos.
 * @returns S3 object record with storage and s3 key
 */
export function parseS3Object(s3Object: S3Object): S3ObjectRecord {
  if (typeof s3Object === "object") return s3Object;
  const match = s3Object.match(/^s3:\/\/([^/]*)\/(.*)$/);
  if (match) return { storage: match[1] || undefined, s3: match[2] ?? "" };
  throw new Error(
    `Invalid s3 object ${JSON.stringify(s3Object)}: expected s3://<storage>/<key> (e.g. "s3:///${s3Object}" for key "${s3Object}" in the default storage) or { s3: <key> }`
  );
}
