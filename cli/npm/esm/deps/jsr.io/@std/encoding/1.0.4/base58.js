// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
/**
 * Utilities for
 * {@link https://datatracker.ietf.org/doc/html/draft-msporny-base58-03 | base58}
 * encoding and decoding.
 *
 * ```ts
 * import { encodeBase58, decodeBase58 } from "@std/encoding/base58";
 * import { assertEquals } from "@std/assert";
 *
 * const hello = new TextEncoder().encode("Hello World!");
 *
 * assertEquals(encodeBase58(hello), "2NEpo7TZRRrLZSi2U");
 *
 * assertEquals(decodeBase58("2NEpo7TZRRrLZSi2U"), hello);
 * ```
 *
 * @module
 */
import { validateBinaryLike } from "./_validate_binary_like.js";
// deno-fmt-ignore
const mapBase58 = {
    "1": 0, "2": 1, "3": 2, "4": 3, "5": 4, "6": 5, "7": 6, "8": 7, "9": 8, A: 9,
    B: 10, C: 11, D: 12, E: 13, F: 14, G: 15, H: 16, J: 17, K: 18, L: 19, M: 20,
    N: 21, P: 22, Q: 23, R: 24, S: 25, T: 26, U: 27, V: 28, W: 29, X: 30, Y: 31,
    Z: 32, a: 33, b: 34, c: 35, d: 36, e: 37, f: 38, g: 39, h: 40, i: 41, j: 42,
    k: 43, m: 44, n: 45, o: 46, p: 47, q: 48, r: 49, s: 50, t: 51, u: 52, v: 53,
    w: 54, x: 55, y: 56, z: 57
};
const base58alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".split("");
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
export function encodeBase58(data) {
    const uint8tData = validateBinaryLike(data);
    let length = 0;
    let zeroes = 0;
    // Counting leading zeroes
    let index = 0;
    while (uint8tData[index] === 0) {
        zeroes++;
        index++;
    }
    const notZeroUint8Data = uint8tData.slice(index);
    const size = Math.round((uint8tData.length * 138) / 100 + 1);
    const b58Encoding = [];
    notZeroUint8Data.forEach((byte) => {
        let i = 0;
        let carry = byte;
        for (let reverseIterator = size - 1; (carry > 0 || i < length) && reverseIterator !== -1; reverseIterator--, i++) {
            carry += (b58Encoding[reverseIterator] ?? 0) * 256;
            b58Encoding[reverseIterator] = Math.round(carry % 58);
            carry = Math.floor(carry / 58);
        }
        length = i;
    });
    const strResult = Array.from({
        length: b58Encoding.length + zeroes,
    });
    if (zeroes > 0) {
        strResult.fill("1", 0, zeroes);
    }
    b58Encoding.forEach((byteValue) => strResult.push(base58alphabet[byteValue]));
    return strResult.join("");
}
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
export function decodeBase58(b58) {
    const splitInput = b58.trim().split("");
    let length = 0;
    let ones = 0;
    // Counting leading ones
    let index = 0;
    while (splitInput[index] === "1") {
        ones++;
        index++;
    }
    const notZeroData = splitInput.slice(index);
    const size = Math.round((b58.length * 733) / 1000 + 1);
    const output = [];
    notZeroData.forEach((char, idx) => {
        let carry = mapBase58[char];
        let i = 0;
        if (carry === undefined) {
            throw new TypeError(`Invalid base58 char at index ${idx} with value ${char}`);
        }
        for (let reverseIterator = size - 1; (carry > 0 || i < length) && reverseIterator !== -1; reverseIterator--, i++) {
            carry += 58 * (output[reverseIterator] ?? 0);
            output[reverseIterator] = Math.round(carry % 256);
            carry = Math.floor(carry / 256);
        }
        length = i;
    });
    const validOutput = output.filter((item) => item !== undefined);
    if (ones > 0) {
        const onesResult = Array.from({ length: ones }).fill(0, 0, ones);
        return new Uint8Array([...onesResult, ...validOutput]);
    }
    return new Uint8Array(validOutput);
}
