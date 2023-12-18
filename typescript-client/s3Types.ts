export type S3Object = {
    s3: string
}

export type DenoS3LightClientSettings = {
    endPoint: string,
    region: string ,
    bucket: string | undefined,
    useSSL: boolean | undefined,
    accessKey: string | undefined,
    secretKey: string | undefined,
    pathStyle: boolean | undefined,
}
