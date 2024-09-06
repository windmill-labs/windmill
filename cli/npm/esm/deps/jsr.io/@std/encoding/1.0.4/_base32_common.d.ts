/**
 * Decodes an encoded string with the given lookup table.
 *
 * @param b32 The string to encode.
 * @param lookup The lookup table
 * @returns The encoded string.
 */
export declare function decode(b32: string, lookup: ReadonlyArray<string>): Uint8Array;
/**
 * Encodes the given data using the lookup table.
 *
 * @param data The data to encode.
 * @param lookup The lookup table.
 * @returns The encoded string.
 */
export declare function encode(data: ArrayBuffer | Uint8Array | string, lookup: ReadonlyArray<string>): string;
//# sourceMappingURL=_base32_common.d.ts.map