// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright 2020 Keith Cirkel. All rights reserved. MIT license.
// Copyright 2023 Skye "MierenManz". All rights reserved. MIT license.
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

// This implementation is a port of https://deno.land/x/varint@v2.0.0 by @keithamus
// This module is browser compatible.

/**
 * The maximum value of an unsigned 64-bit integer.
 * Equivalent to `2n**64n - 1n`
 */
export const MaxUint64 = 18446744073709551615n;

/**
 * The maximum length, in bytes, of a Varint encoded 64-bit integer.
 */
export const MaxVarintLen64 = 10;
/**
 * The maximum length, in bytes, of a Varint encoded 32-bit integer.
 */
export const MaxVarintLen32 = 5;

const MSB = 0x80;
const REST = 0x7f;
const SHIFT = 7;
const MSBN = 0x80n;
const SHIFTN = 7n;

// ArrayBuffer and TypedArray's for "pointer casting"
const AB = new ArrayBuffer(8);
const U32_VIEW = new Uint32Array(AB);
const U64_VIEW = new BigUint64Array(AB);

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
export function decodeVarint(buf: Uint8Array, offset = 0): [bigint, number] {
  // Clear the last result from the Two's complement view
  U64_VIEW[0] = 0n;

  // Setup the initiat state of the function
  let intermediate = 0;
  let position = 0;
  let i = offset;

  // If the buffer is empty Throw
  if (buf.length === 0) throw new RangeError("Cannot read empty buffer");

  let byte;
  do {
    // Get a single byte from the buffer
    byte = buf[i]!;

    // 1. Take the lower 7 bits of the byte.
    // 2. Shift the bits into the correct position.
    // 3. Bitwise OR it with the intermediate value
    // QUIRK: in the 5th (and 10th) iteration of this loop it will overflow on the shift.
    // This causes only the lower 4 bits to be shifted into place and removing the upper 3 bits
    intermediate |= (byte & 0b01111111) << position;

    // If position is 28
    // it means that this iteration needs to be written the the two's complement view
    // This only happens once due to the `-4` in this branch
    if (position === 28) {
      // Write to the view
      U32_VIEW[0] = intermediate;
      // set `intermediate` to the remaining 3 bits
      // We only want the remaining three bits because the other 4 have been "consumed" on line 21
      intermediate = (byte & 0b01110000) >>> 4;
      // set `position` to -4 because later 7 will be added, making it 3
      position = -4;
    }

    // Increment the shift position by 7
    position += 7;
    // Increment the iterator by 1
    i++;
    // Keep going while there is a continuation bit
  } while ((byte & 0b10000000) === 0b10000000);
  // subtract the initial offset from `i` to get the bytes read
  const nRead = i - offset;

  // If 10 bytes have been read and intermediate has overflown
  // it means that the varint is malformed
  // If 11 bytes have been read it means that the varint is malformed
  // If `i` is bigger than the buffer it means we overread the buffer and the varint is malformed
  if ((nRead === 10 && intermediate > -1) || nRead === 11 || i > buf.length) {
    throw new RangeError(
      "Cannot decode the varint input: Malformed or overflow varint",
    );
  }

  // Write the intermediate value to the "empty" slot
  // if the first slot is taken. Take the second slot
  U32_VIEW[Number(nRead > 4)] = intermediate;

  return [U64_VIEW[0], i];
}

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
export function decodeVarint32(buf: Uint8Array, offset = 0): [number, number] {
  let shift = 0;
  let decoded = 0;
  for (
    let i = offset;
    i <= Math.min(buf.length, offset + MaxVarintLen32);
    i += 1, shift += SHIFT
  ) {
    const byte = buf[i]!;
    decoded += (byte & REST) * Math.pow(2, shift);
    if (!(byte & MSB)) return [decoded, i + 1];
  }
  throw new RangeError(
    "Cannot decode the varint input: Malformed or overflow varint",
  );
}

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
export function encodeVarint(
  num: bigint | number,
  buf: Uint8Array = new Uint8Array(MaxVarintLen64),
  offset = 0,
): [Uint8Array, number] {
  num = BigInt(num);
  if (num < 0n) {
    throw new RangeError(
      `Cannot encode the input into varint as it should be non-negative integer: received ${num}`,
    );
  }
  for (
    let i = offset;
    i <= Math.min(buf.length, MaxVarintLen64);
    i += 1
  ) {
    if (num < MSBN) {
      buf[i] = Number(num);
      i += 1;
      return [buf.slice(offset, i), i];
    }
    buf[i] = Number((num & 0xFFn) | MSBN);
    num >>= SHIFTN;
  }
  throw new RangeError(
    `Cannot encode the input ${num} into varint as it overflows uint64`,
  );
}
