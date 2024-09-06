"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.assert = void 0;
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
const assertion_error_js_1 = require("./assertion_error.js");
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
function assert(expr, msg = "") {
    if (!expr) {
        throw new assertion_error_js_1.AssertionError(msg);
    }
}
exports.assert = assert;
