/**
 * Supported ascii85 standards for {@linkcode EncodeAscii85Options} and
 * {@linkcode DecodeAscii85Options}.
 */
export type Ascii85Standard = "Adobe" | "btoa" | "RFC 1924" | "Z85";
/** Options for {@linkcode encodeAscii85}. */
export interface EncodeAscii85Options {
    /**
     * Character set and delimiter (if supported and used).
     *
     * @default {"Adobe"}
     */
    standard?: Ascii85Standard;
    /**
     * Whether to use a delimiter (if supported).
     *
     * @default {false}
     */
    delimiter?: boolean;
}
/**
 * Converts data into an ascii85-encoded string.
 *
 * @param data The data to encode.
 * @param options Options for encoding.
 *
 * @returns The ascii85-encoded string.
 *
 * @example Usage
 * ```ts
 * import { encodeAscii85 } from "@std/encoding/ascii85";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(encodeAscii85("Hello world!"), "87cURD]j7BEbo80");
 * ```
 */
export declare function encodeAscii85(data: ArrayBuffer | Uint8Array | string, options?: EncodeAscii85Options): string;
/** Options for {@linkcode decodeAscii85}. */
export type DecodeAscii85Options = Omit<EncodeAscii85Options, "delimiter">;
/**
 * Decodes a ascii85-encoded string.
 *
 * @param ascii85 The ascii85-encoded string to decode.
 * @param options Options for decoding.
 * @returns The decoded data.
 *
 * @example Usage
 * ```ts
 * import { decodeAscii85 } from "@std/encoding/ascii85";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(
 *   decodeAscii85("87cURD]j7BEbo80"),
 *   new TextEncoder().encode("Hello world!"),
 * );
 * ```
 */
export declare function decodeAscii85(ascii85: string, options?: DecodeAscii85Options): Uint8Array;
//# sourceMappingURL=ascii85.d.ts.map