import { HttpResponse } from "@smithy/protocol-http";
import { PreviouslyResolved } from "./configuration";
export interface ValidateChecksumFromResponseOptions {
    config: PreviouslyResolved;
    /**
     * Defines the checksum algorithms clients SHOULD look for when validating checksums
     * returned in the HTTP response.
     */
    responseAlgorithms?: string[];
}
export declare const validateChecksumFromResponse: (response: HttpResponse, { config, responseAlgorithms }: ValidateChecksumFromResponseOptions) => Promise<void>;
