import { ChecksumConstructor, Encoder, HashConstructor, StreamHasher } from "@smithy/types";
export interface GetChecksumDigestOptions {
    streamHasher: StreamHasher<any>;
    checksumAlgorithmFn: ChecksumConstructor | HashConstructor;
    base64Encoder: Encoder;
}
export declare const getChecksum: (body: unknown, { streamHasher, checksumAlgorithmFn, base64Encoder }: GetChecksumDigestOptions) => Promise<string>;
