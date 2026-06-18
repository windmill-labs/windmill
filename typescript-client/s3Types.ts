/**
 * S3 object representation, either as a URI string or a record object
 */
export type S3Object = S3ObjectURI | S3ObjectRecord;

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
