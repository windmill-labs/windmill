// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

/**
 * Utilities for encoding and decoding common formats like hex, base64, and varint.
 *
 * ```ts
 * import { encodeBase64, decodeBase64 } from "@std/encoding";
 * import { assertEquals } from "@std/assert";
 *
 * const foobar = new TextEncoder().encode("foobar");
 * assertEquals(encodeBase64(foobar), "Zm9vYmFy");
 * assertEquals(decodeBase64("Zm9vYmFy"), foobar);
 * ```
 *
 * @module
 */

export * from "./ascii85.js";
export * from "./base32.js";
export * from "./base32_stream.js";
export * from "./base32hex.js";
export * from "./base32hex_stream.js";
export * from "./base58.js";
export * from "./base64.js";
export * from "./base64_stream.js";
export * from "./base64url.js";
export * from "./base64url_stream.js";
export * from "./hex.js";
export * from "./hex_stream.js";
export * from "./varint.js";
