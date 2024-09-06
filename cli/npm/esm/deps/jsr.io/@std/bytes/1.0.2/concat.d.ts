/**
 * Concatenate an array of byte slices into a single slice.
 *
 * @param buffers Array of byte slices to concatenate.
 * @returns A new byte slice containing all the input slices concatenated.
 *
 * @example Basic usage
 * ```ts
 * import { concat } from "@std/bytes/concat";
 * import { assertEquals } from "@std/assert";
 *
 * const a = new Uint8Array([0, 1, 2]);
 * const b = new Uint8Array([3, 4, 5]);
 *
 * assertEquals(concat([a, b]), new Uint8Array([0, 1, 2, 3, 4, 5]));
 * ```
 */
export declare function concat(buffers: Uint8Array[]): Uint8Array;
//# sourceMappingURL=concat.d.ts.map