// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import { AssertionError } from "./assertion_error.js";
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
export function assert(expr, msg = "") {
    if (!expr) {
        throw new AssertionError(msg);
    }
}
