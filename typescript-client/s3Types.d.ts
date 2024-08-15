export type S3Object = {
    s3: string;
    storage?: string;
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
