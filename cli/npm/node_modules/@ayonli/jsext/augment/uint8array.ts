import {
    compare,
    equals as _equals,
    includesSlice as _includesSlice,
    startsWith as _startsWith,
    endsWith as _endsWith,
    split as _split,
    chunk as _chunk,
    concat,
    copy,
} from "../bytes.ts";

declare global {
    interface Uint8ArrayConstructor {
        /** Copies bytes from `src` array to `dest` and returns the number of bytes copied. */
        copy(src: Uint8Array, dest: Uint8Array): number;
        /** Like `Buffer.concat` but for native `Uint8Array`. */
        concat<T extends Uint8Array>(...arrays: T[]): T;
        /** Like `Buffer.compare` but for native `Uint8Array`. */
        compare(arr1: Uint8Array, arr2: Uint8Array): -1 | 0 | 1;
    }

    interface Uint8Array {
        /** Checks if the two byte arrays are equal to each other. */
        equals(another: Uint8Array): boolean;
        /** Checks if the byte array contains another array as a slice of its contents. */
        includesSlice(subset: Uint8Array): boolean;
        /** Checks if the byte array starts with the given prefix. */
        startsWith(prefix: Uint8Array): boolean;
        /** Checks if the byte array ends with the given suffix. */
        endsWith(suffix: Uint8Array): boolean;
        /** Breaks the byte array into smaller chunks according to the given delimiter. */
        split(delimiter: number): this[];
        /** Breaks the byte array into smaller chunks according to the given length. */
        chunk(length: number): this[];
    }
}

Uint8Array.copy = copy;
Uint8Array.concat = concat;
Uint8Array.compare = compare;

Uint8Array.prototype.equals = function equals(another) {
    return _equals(this, another);
};

Uint8Array.prototype.includesSlice = function includesSlice(slice) {
    return _includesSlice(this, slice);
};

Uint8Array.prototype.startsWith = function startsWith(prefix) {
    return _startsWith(this, prefix);
};

Uint8Array.prototype.endsWith = function endsWith(suffix) {
    return _endsWith(this, suffix);
};

Uint8Array.prototype.split = function split(delimiter) {
    return _split(this, delimiter);
};

Uint8Array.prototype.chunk = function chunk(length) {
    return _chunk(this, length);
};
