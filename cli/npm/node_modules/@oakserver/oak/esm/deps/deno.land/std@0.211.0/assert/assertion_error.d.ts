/**
 * Error thrown when an assertion fails.
 *
 * @example
 * ```ts
 * import { AssertionError } from "https://deno.land/std@$STD_VERSION/assert/assertion_error.ts";
 *
 * throw new AssertionError("Assertion failed");
 * ```
 */
export declare class AssertionError extends Error {
    /** Constructs a new instance. */
    constructor(message: string);
}
