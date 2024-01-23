import { DEFAULT_CHECKSUM_ALGORITHM, S3_EXPRESS_DEFAULT_CHECKSUM_ALGORITHM } from "./constants";
import { CLIENT_SUPPORTED_ALGORITHMS } from "./types";
export const getChecksumAlgorithmForRequest = (input, { requestChecksumRequired, requestAlgorithmMember }, isS3Express) => {
    const defaultAlgorithm = isS3Express ? S3_EXPRESS_DEFAULT_CHECKSUM_ALGORITHM : DEFAULT_CHECKSUM_ALGORITHM;
    if (!requestAlgorithmMember || !input[requestAlgorithmMember]) {
        return requestChecksumRequired ? defaultAlgorithm : undefined;
    }
    const checksumAlgorithm = input[requestAlgorithmMember];
    if (!CLIENT_SUPPORTED_ALGORITHMS.includes(checksumAlgorithm)) {
        throw new Error(`The checksum algorithm "${checksumAlgorithm}" is not supported by the client.` +
            ` Select one of ${CLIENT_SUPPORTED_ALGORITHMS}.`);
    }
    return checksumAlgorithm;
};
