// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
/**
 * Concatenate an array of {@linkcode Uint8Array}s.
 *
 * @example
 * ```ts
 * import { concat } from "https://deno.land/std@$STD_VERSION/bytes/concat.ts";
 *
 * const a = new Uint8Array([0, 1, 2]);
 * const b = new Uint8Array([3, 4, 5]);
 * concat([a, b]); // Uint8Array(6) [ 0, 1, 2, 3, 4, 5 ]
 * ```
 */
export function concat(buf) {
    let length = 0;
    for (const b of buf) {
        length += b.length;
    }
    const output = new Uint8Array(length);
    let index = 0;
    for (const b of buf) {
        output.set(b, index);
        index += b.length;
    }
    return output;
}
