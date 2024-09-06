/**
 * Make an assertion, error will be thrown if `expr` does not have truthy value.
 *
 * @example
 * ```ts
 * import { assert } from "https://deno.land/std@$STD_VERSION/assert/assert.ts";
 *
 * assert("hello".includes("ello")); // Doesn't throw
 * assert("hello".includes("world")); // Throws
 * ```
 */
export declare function assert(expr: unknown, msg?: string): asserts expr;
