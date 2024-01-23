import { ChecksumAlgorithm } from "./constants";
export interface GetChecksumAlgorithmForRequestOptions {
  requestChecksumRequired: boolean;
  requestAlgorithmMember?: string;
}
export declare const getChecksumAlgorithmForRequest: (
  input: any,
  {
    requestChecksumRequired,
    requestAlgorithmMember,
  }: GetChecksumAlgorithmForRequestOptions,
  isS3Express?: boolean
) => ChecksumAlgorithm | undefined;
