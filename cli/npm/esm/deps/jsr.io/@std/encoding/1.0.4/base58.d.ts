/**
 * Converts data into a base58-encoded string.
 *
 * @see {@link https://datatracker.ietf.org/doc/html/draft-msporny-base58-03#section-3}
 *
 * @param data The data to encode.
 * @returns The base58-encoded string.
 *
 * @example Usage
 * ```ts
 * import { encodeBase58 } from "@std/encoding/base58";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(encodeBase58("Hello World!"), "2NEpo7TZRRrLZSi2U");
 * ```
 */
export declare function encodeBase58(data: ArrayBuffer | Uint8Array | string): string;
/**
 * Decodes a base58-encoded string.
 *
 * @see {@link https://datatracker.ietf.org/doc/html/draft-msporny-base58-03#section-4}
 *
 * @param b58 The base58-encoded string to decode.
 * @returns The decoded data.
 *
 * @example Usage
 * ```ts
 * import { decodeBase58 } from "@std/encoding/base58";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(
 *   decodeBase58("2NEpo7TZRRrLZSi2U"),
 *   new TextEncoder().encode("Hello World!")
 * );
 * ```
 */
export declare function decodeBase58(b58: string): Uint8Array;
//# sourceMappingURL=base58.d.ts.map