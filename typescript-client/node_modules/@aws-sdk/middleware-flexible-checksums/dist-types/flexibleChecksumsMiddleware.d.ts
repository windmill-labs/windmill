import { BuildHandlerOptions, BuildMiddleware } from "@smithy/types";
import { PreviouslyResolved } from "./configuration";
export interface FlexibleChecksumsRequestMiddlewareConfig {
    /**
     * The input object for the operation.
     */
    input: Object;
    /**
     * Indicates an operation requires a checksum in its HTTP request.
     */
    requestChecksumRequired: boolean;
    /**
     * Defines a top-level operation input member that is used to configure request checksum behavior.
     */
    requestAlgorithmMember?: string;
}
export declare const flexibleChecksumsMiddlewareOptions: BuildHandlerOptions;
/**
 * @internal
 */
export declare const flexibleChecksumsMiddleware: (config: PreviouslyResolved, middlewareConfig: FlexibleChecksumsRequestMiddlewareConfig) => BuildMiddleware<any, any>;
