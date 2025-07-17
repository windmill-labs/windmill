export type S3Object = S3ObjectURI | S3ObjectRecord;

export type S3ObjectURI = `s3://${string}/${string}`;
export type S3ObjectRecord = {
  s3: string;
  storage?: string;
  presigned?: string;
};

export type DenoS3LightClientSettings = {
  endPoint: string;
  region: string;
  bucket?: string;
  useSSL?: boolean;
  accessKey?: string;
  secretKey?: string;
  pathStyle?: boolean;
};
