// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright (c) 2014 Jameson Little. MIT License.
// This module is browser compatible.
/**
 * Utilities for
 * {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-6 | base32}
 * encoding and decoding.
 *
 * Modified from {@link https://github.com/beatgammit/base64-js}.
 *
 * ```ts
 * import { encodeBase32, decodeBase32 } from "@std/encoding/base32";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(encodeBase32("foobar"), "MZXW6YTBOI======");
 *
 * assertEquals(
 *   decodeBase32("MZXW6YTBOI======"),
 *   new TextEncoder().encode("foobar")
 * );
 * ```
 *
 * @module
 */
import { decode, encode } from "./_base32_common.js";
const lookup = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".split("");
const revLookup = [];
lookup.forEach((c, i) => (revLookup[c.charCodeAt(0)] = i));
/**
 * Decodes a base32-encoded string.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-6}
 *
 * @param b32 The base32-encoded string to decode.
 * @returns The decoded data.
 *
 * @example Usage
 * ```ts
 * import { decodeBase32 } from "@std/encoding/base32";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(
 *   decodeBase32("GZRTMMDDGA======"),
 *   new TextEncoder().encode("6c60c0"),
 * );
 * ```
 */
export function decodeBase32(b32) {
    return decode(b32, lookup);
}
/**
 * Converts data into a base32-encoded string.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-6}
 *
 * @param data The data to encode.
 * @returns The base32-encoded string.
 *
 * @example Usage
 * ```ts
 * import { encodeBase32 } from "@std/encoding/base32";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(encodeBase32("6c60c0"), "GZRTMMDDGA======");
 * ```
 */
export function encodeBase32(data) {
    return encode(data, lookup);
}
