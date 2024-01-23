import { HttpResponse } from "@smithy/protocol-http";
import { PreviouslyResolved } from "./configuration";
export interface ValidateChecksumFromResponseOptions {
  config: PreviouslyResolved;
  responseAlgorithms?: string[];
}
export declare const validateChecksumFromResponse: (
  response: HttpResponse,
  { config, responseAlgorithms }: ValidateChecksumFromResponseOptions
) => Promise<void>;
