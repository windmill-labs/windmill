import { ChecksumAlgorithm } from "./constants";
export interface GetChecksumAlgorithmForRequestOptions {
    /**
     * Indicates an operation requires a checksum in its HTTP request.
     */
    requestChecksumRequired: boolean;
    /**
     * Defines a top-level operation input member that is used to configure request checksum behavior.
     */
    requestAlgorithmMember?: string;
}
/**
 * Returns the checksum algorithm to use for the request, along with
 * the priority array of location to use to populate checksum and names
 * to be used as a key at the location.
 */
export declare const getChecksumAlgorithmForRequest: (input: any, { requestChecksumRequired, requestAlgorithmMember }: GetChecksumAlgorithmForRequestOptions, isS3Express?: boolean) => ChecksumAlgorithm | undefined;
