/**
 * Functions for dealing with byte arrays (`Uint8Array`).
 * @module
 */

import {
    equals as _equals,
    includeSlice as _includesSlice,
    startsWith as _startsWith,
    endsWith as _endsWith,
    split as _split,
    chunk as _chunk,
} from "./array/base.ts";
import { decodeBase64, decodeHex, encodeBase64, encodeHex } from "./encoding.ts";
import { sum } from "./math.ts";
import { Constructor } from "./types.ts";

const defaultEncoder = new TextEncoder();
const defaultDecoder = new TextDecoder();

/**
 * A byte array is a `Uint8Array` that can be coerced to a string with `utf8`
 * encoding.
 */
export class ByteArray extends Uint8Array {
    override toString(): string {
        return text(this);
    }

    toJSON(): { type: "ByteArray"; data: number[]; } {
        return {
            type: "ByteArray",
            data: Array.from(this),
        };
    }
}

/**
 * Converts the given data to a byte array.
 * 
 * @example
 * ```ts
 * import bytes from "@ayonli/jsext/bytes";
 * 
 * const arr = bytes("Hello, World!");
 * 
 * console.log(arr);
 * // ByteArray(13) [Uint8Array] [ 72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33 ]
 * 
 * console.log(String(arr)); // "Hello, World!"
 * 
 * // from hex
 * const arr2 = bytes("48656c6c6f2c20576f726c6421", "hex");
 * 
 * // from base64
 * const arr3 = bytes("SGVsbG8sIFdvcmxkIQ==", "base64");
 * ```
 */
export default function bytes(str: string, encoding?: "utf8" | "hex" | "base64"): ByteArray;
export default function bytes(arr: string | ArrayBufferLike | ArrayBufferView | ArrayLike<number>): ByteArray;
/**
 * Creates a byte array with the specified length.
 * 
 * @example
 * ```ts
 * import bytes from "@ayonli/jsext/bytes";
 * 
 * const arr = bytes(10);
 * 
 * console.log(arr);
 * // ByteArray(10) [Uint8Array] [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ]
 * ```
 */
export default function bytes(length: number): ByteArray;
export default function bytes(
    data: number | string | ArrayBufferLike | ArrayBufferView | ArrayLike<number>,
    encoding: "utf8" | "hex" | "base64" = "utf8"
): ByteArray {
    if (typeof data === "number") {
        return new ByteArray(data);
    } else if (typeof data === "string") {
        let _data: Uint8Array;

        if (encoding === "hex") {
            _data = decodeHex(data);
        } else if (encoding === "base64") {
            _data = decodeBase64(data);
        } else {
            _data = defaultEncoder.encode(data);
        }

        return new ByteArray(_data.buffer, _data.byteOffset, _data.byteLength);
    } else if (ArrayBuffer.isView(data)) {
        return new ByteArray(data.buffer, data.byteOffset, data.byteLength);
    } else {
        return new ByteArray(data);
    }
}

/**
 * Converts the byte array (or `Uint8Array`) to a string.
 * @param encoding Default value: `utf8`.
 * 
 * @example
 * ```ts
 * import { text } from "@ayonli/jsext/bytes";
 * 
 * const arr = new Uint8Array([72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33]);
 * 
 * console.log(text(arr)); // "Hello, World!"
 * console.log(text(arr, "hex")); // "48656c6c6f2c20576f726c6421"
 * console.log(text(arr, "base64")); // "SGVsbG8sIFdvcmxkIQ=="
 * ```
 */
export function text(
    bytes: Uint8Array,
    encoding: "utf8" | "hex" | "base64" = "utf8"
): string {
    if (encoding === "hex") {
        return encodeHex(bytes);
    } else if (encoding === "base64") {
        return encodeBase64(bytes);
    } else if (typeof Buffer === "function" && bytes instanceof Buffer) {
        return bytes.toString("utf8");
    } else {
        return defaultDecoder.decode(bytes);
    }
}

/**
 * Copies bytes from `src` array to `dest` and returns the number of bytes copied.
 * 
 * @example
 * ```ts
 * import { copy } from "@ayonli/jsext/bytes";
 * 
 * const src = new Uint8Array([1, 2, 3, 4, 5]);
 * const dest = new Uint8Array(3);
 * 
 * const n = copy(src, dest);
 * 
 * console.log(n); // 3
 * console.log(dest); // Uint8Array(3) [ 1, 2, 3 ]
 * ```
 */
export function copy(src: Uint8Array, dest: Uint8Array): number {
    if (src.length > dest.length) {
        src = src.subarray(0, dest.length);
    }

    dest.set(src);
    return src.length;
}

/**
 * Like `Buffer.concat` but for native `Uint8Array`.
 * 
 * @example
 * ```ts
 * import { concat } from "@ayonli/jsext/bytes";
 * 
 * const arr1 = new Uint8Array([1, 2, 3]);
 * const arr2 = new Uint8Array([4, 5, 6]);
 * 
 * const result = concat(arr1, arr2);
 * 
 * console.log(result); // Uint8Array(6) [ 1, 2, 3, 4, 5, 6 ]
 * ```
 */
export function concat<T extends Uint8Array>(...arrays: T[]): T {
    const length = sum(...arrays.map(arr => arr.length));
    const ctor = ((arrays[0] as T)?.constructor || Uint8Array) as Constructor<T>;
    const result = typeof Buffer === "function" && Object.is(ctor, Buffer)
        ? Buffer.alloc(length) as unknown as T
        : new ctor(length);
    let offset = 0;

    for (const arr of arrays) {
        result.set(arr, offset);
        offset += arr.length;
    }

    return result;
}

/**
 * Like `Buffer.compare` but for native `Uint8Array`.
 * 
 * @example
 * ```ts
 * import { compare } from "@ayonli/jsext/bytes";
 * 
 * const arr1 = new Uint8Array([1, 2, 3]);
 * const arr2 = new Uint8Array([1, 2, 4]);
 * const arr3 = new Uint8Array([1, 2, 3, 4]);
 * const arr4 = new Uint8Array([1, 2, 3]);
 * const arr5 = new Uint8Array([1, 2]);
 * 
 * console.log(compare(arr1, arr2)); // -1
 * console.log(compare(arr1, arr3)); // -1
 * console.log(compare(arr1, arr4)); // 0
 * console.log(compare(arr1, arr5)); // 1
 * ```
 */
export function compare(arr1: Uint8Array, arr2: Uint8Array): -1 | 0 | 1 {
    if (arr1 === arr2) {
        return 0;
    }

    for (let i = 0; i < arr1.length; i++) {
        const ele1 = arr1[i] as number;
        const ele2 = arr2[i];

        if (ele2 === undefined) {
            return 1;
        } else if (ele1 < ele2) {
            return -1;
        } else if (ele1 > ele2) {
            return 1;
        }
    }

    return arr1.length < arr2.length ? -1 : 0;
}

/**
 * Checks if the two byte arrays are equal to each other.
 * 
 * @example
 * ```ts
 * import { equals } from "@ayonli/jsext/bytes";
 * 
 * const arr1 = new Uint8Array([1, 2, 3]);
 * const arr2 = new Uint8Array([1, 2, 3]);
 * const arr3 = new Uint8Array([1, 2, 4]);
 * 
 * console.log(equals(arr1, arr2)); // true
 * console.log(equals(arr1, arr3)); // false
 * ```
 */
export function equals(arr1: Uint8Array, arr2: Uint8Array): boolean {
    if (arr1 === arr2) {
        return true;
    } else if (arr1.length !== arr2.length) {
        return false;
    } else if (arr1.length < 1000) {
        return _equals(arr1, arr2);
    }

    const len = arr1.length;
    const compressible = Math.floor(len / 4);
    const _arr1 = new Uint32Array(arr1.buffer, 0, compressible);
    const _arr2 = new Uint32Array(arr2.buffer, 0, compressible);

    for (let i = compressible * 4; i < len; i++) {
        if (arr1[i] !== arr2[i]) {
            return false;
        }
    }

    for (let i = 0; i < _arr1.length; i++) {
        if (_arr1[i] !== _arr2[i]) {
            return false;
        }
    }

    return true;
}

/**
 * Checks if the byte array contains another array as a slice of its contents.
 * 
 * @example
 * ```ts
 * import { includesSlice } from "@ayonli/jsext/bytes";
 * 
 * const arr = new Uint8Array([1, 2, 3, 4, 5]);
 * 
 * console.log(includesSlice(arr, new Uint8Array([3, 4]))); // true
 * console.log(includesSlice(arr, new Uint8Array([4, 3]))); // false
 * ```
 */
export function includesSlice(arr: Uint8Array, slice: Uint8Array): boolean {
    return _includesSlice(arr, slice);
}

/**
 * Checks if the byte array starts with the given prefix.
 * 
 * @example
 * ```ts
 * import { startsWith } from "@ayonli/jsext/bytes";
 * 
 * const arr = new Uint8Array([1, 2, 3, 4, 5]);
 * 
 * console.log(startsWith(arr, new Uint8Array([1, 2]))); // true
 * console.log(startsWith(arr, new Uint8Array([2, 1]))); // false
 * ```
 */
export function startsWith(arr: Uint8Array, prefix: Uint8Array): boolean {
    return _startsWith(arr, prefix);
}

/**
 * Checks if the byte array ends with the given suffix.
 * 
 * @example
 * ```ts
 * import { endsWith } from "@ayonli/jsext/bytes";
 * 
 * const arr = new Uint8Array([1, 2, 3, 4, 5]);
 * 
 * console.log(endsWith(arr, new Uint8Array([4, 5]))); // true
 * console.log(endsWith(arr, new Uint8Array([5, 4]))); // false
 * ```
 */
export function endsWith(arr: Uint8Array, suffix: Uint8Array): boolean {
    return _endsWith(arr, suffix);
}

/**
 * Breaks the byte array into smaller chunks according to the given delimiter.
 * 
 * @example
 * ```ts
 * import { split } from "@ayonli/jsext/bytes";
 * 
 * const arr = new Uint8Array([1, 2, 3, 0, 4, 5, 0, 6, 7]);
 * 
 * console.log(split(arr, 0));
 * // [
 * //     Uint8Array(3) [ 1, 2, 3 ],
 * //     Uint8Array(2) [ 4, 5 ],
 * //     Uint8Array(2) [ 6, 7 ]
 * // ]
 * ```
 */
export function split<T extends Uint8Array>(arr: T, delimiter: number): T[] {
    return _split(arr, delimiter) as T[];
}

/**
 * Breaks the byte array into smaller chunks according to the given length.
 * 
 * @example
 * ```ts
 * import { chunk } from "@ayonli/jsext/bytes";
 * 
 * const arr = new Uint8Array([1, 2, 3, 4, 5, 6, 7]);
 * 
 * console.log(chunk(arr, 3));
 * // [
 * //     Uint8Array(3) [ 1, 2, 3 ],
 * //     Uint8Array(3) [ 4, 5, 6 ],
 * //     Uint8Array(1) [ 7 ]
 * // ]
 * ```
 */
export function chunk<T extends Uint8Array>(arr: T, length: number): T[] {
    return _chunk(arr, length) as T[];
}
