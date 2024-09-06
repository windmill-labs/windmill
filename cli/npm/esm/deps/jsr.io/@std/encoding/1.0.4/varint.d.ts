/**
 * Utilities for {@link https://protobuf.dev/programming-guides/encoding/#varints Varint} encoding
 * of typed integers. Varint encoding represents integers using a variable number of bytes, with
 * smaller values requiring fewer bytes.
 *
 * ```ts
 * import { encodeVarint, decodeVarint } from "@std/encoding/varint";
 * import { assertEquals } from "@std/assert";
 *
 * const buf = new Uint8Array(10);
 * assertEquals(
 *   encodeVarint(42n, buf),
 *   [new Uint8Array([42]), 1]
 * );
 *
 * assertEquals(
 *   decodeVarint(new Uint8Array([42])),
 *   [ 42n, 1 ]
 * );
 * ```
 *
 * @module
 */
/**
 * The maximum value of an unsigned 64-bit integer.
 * Equivalent to `2n**64n - 1n`
 */
export declare const MaxUint64 = 18446744073709551615n;
/**
 * The maximum length, in bytes, of a Varint encoded 64-bit integer.
 */
export declare const MaxVarintLen64 = 10;
/**
 * The maximum length, in bytes, of a Varint encoded 32-bit integer.
 */
export declare const MaxVarintLen32 = 5;
/**
 * Given a non empty `buf`, starting at `offset` (default: 0), begin decoding bytes as
 * Varint encoded bytes, for a maximum of 10 bytes (offset + 10). The returned
 * tuple is of the decoded varint 32-bit number, and the new offset with which
 * to continue decoding other data.
 *
 * If a `bigint` in return is undesired, the `decode32` function will return a
 * `number`, but this should only be used in cases where the varint is
 * _assured_ to be 32-bits. If in doubt, use `decode()`.
 *
 * To know how many bytes the Varint took to encode, simply negate `offset`
 * from the returned new `offset`.
 *
 * @param buf The buffer to decode from.
 * @param offset The offset to start decoding from.
 * @returns A tuple of the decoded varint 64-bit number, and the new offset.
 *
 * @example Usage
 * ```ts
 * import { decodeVarint } from "@std/encoding/varint";
 * import { assertEquals } from "@std/assert";
 *
 * const buf = new Uint8Array([0x8E, 0x02]);
 * assertEquals(decodeVarint(buf), [270n, 2]);
 * ```
 */
export declare function decodeVarint(buf: Uint8Array, offset?: number): [bigint, number];
/**
 * Given a `buf`, starting at `offset` (default: 0), begin decoding bytes as
 * Varint encoded bytes, for a maximum of 5 bytes (offset + 5). The returned
 * tuple is of the decoded varint 32-bit number, and the new offset with which
 * to continue decoding other data.
 *
 * Varints are _not 32-bit by default_ so this should only be used in cases
 * where the varint is _assured_ to be 32-bits. If in doubt, use `decode()`.
 *
 * To know how many bytes the Varint took to encode, simply negate `offset`
 * from the returned new `offset`.
 *
 * @param buf The buffer to decode from.
 * @param offset The offset to start decoding from.
 * @returns A tuple of the decoded varint 32-bit number, and the new offset.
 *
 * @example Usage
 * ```ts
 * import { decodeVarint32 } from "@std/encoding/varint";
 * import { assertEquals } from "@std/assert";
 *
 * const buf = new Uint8Array([0x8E, 0x02]);
 * assertEquals(decodeVarint32(buf), [270, 2]);
 * ```
 */
export declare function decodeVarint32(buf: Uint8Array, offset?: number): [number, number];
/**
 * Takes unsigned number `num` and converts it into a Varint encoded
 * `Uint8Array`, returning a tuple consisting of a `Uint8Array` slice of the
 * encoded Varint, and an offset where the Varint encoded bytes end within the
 * `Uint8Array`.
 *
 * If `buf` is not given then a Uint8Array will be created.
 * `offset` defaults to `0`.
 *
 * If passed `buf` then that will be written into, starting at `offset`. The
 * resulting returned `Uint8Array` will be a slice of `buf`. The resulting
 * returned number is effectively `offset + bytesWritten`.
 *
 * @param num The number to encode.
 * @param buf The buffer to write into.
 * @param offset The offset to start writing at.
 * @returns A tuple of the encoded Varint `Uint8Array` and the new offset.
 *
 * @example Usage
 * ```ts
 * import { encodeVarint } from "@std/encoding/varint";
 * import { assertEquals } from "@std/assert";
 *
 * const buf = new Uint8Array(10);
 * assertEquals(encodeVarint(42n, buf), [new Uint8Array([42]), 1]);
 * ```
 */
export declare function encodeVarint(num: bigint | number, buf?: Uint8Array, offset?: number): [Uint8Array, number];
//# sourceMappingURL=varint.d.ts.map