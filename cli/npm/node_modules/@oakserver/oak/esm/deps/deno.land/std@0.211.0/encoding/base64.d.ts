/**
 * Converts data into a base64-encoded string.
 *
 * @see {@link https://datatracker.ietf.org/doc/html/rfc4648#section-4}
 *
 * @example
 * ```ts
 * import { encodeBase64 } from "https://deno.land/std@$STD_VERSION/encoding/base64.ts";
 *
 * encodeBase64("foobar"); // "Zm9vYmFy"
 * ```
 */
export declare function encodeBase64(data: ArrayBuffer | Uint8Array | string): string;
/**
 * Decodes a base64-encoded string.
 *
 * @see {@link https://datatracker.ietf.org/doc/html/rfc4648#section-4}
 *
 * @example
 * ```ts
 * import { encodeBase64 } from "https://deno.land/std@$STD_VERSION/encoding/base64.ts";
 *
 * encodeBase64("foobar"); // "Zm9vYmFy"
 * ```
 */
export declare function decodeBase64(b64: string): Uint8Array;
