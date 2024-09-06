import { RealArrayLike } from "../types.ts";

/** Counts the occurrence of the element in the array-like object. */
export function count<T>(arr: RealArrayLike<T>, item: T): number {
    let count = 0;

    for (let i = 0; i < arr.length; i++) {
        if (arr[i] === item) {
            count++;
        }
    }

    return count;
}

/**
 * Performs a shallow compare to another array and see if it contains the same elements as
 * this array-like object.
 */
export function equals<T>(arr1: RealArrayLike<T>, arr2: RealArrayLike<T>): boolean {
    if (arr1 === arr2) {
        return true;
    } else if (arr1.length !== arr2.length) {
        return false;
    }

    for (let i = 0; i < arr1.length; i++) {
        if (arr1[i] !== arr2[i]) {
            return false;
        }
    }

    return true;
}

/** Checks if the array-like object contains another array as a slice of its contents. */
export function includeSlice<T>(arr: RealArrayLike<T>, slice: RealArrayLike<T>): boolean {
    if (arr === slice || !slice.length) {
        return true;
    }

    const limit = arr.length;
    const subLimit = slice.length;

    if (subLimit > limit) {
        return false;
    }

    for (let i = 0; i < limit; i++) {
        if (arr[i] === slice[0]) {
            let j = 1;

            while (j < subLimit && arr[i + j] === slice[j]) {
                j++;
            }

            if (j === subLimit) {
                return true;
            }
        }
    }

    return false;
}

/** Checks if the array-like object starts with the given prefix. */
export function startsWith<T>(arr: RealArrayLike<T>, prefix: RealArrayLike<T>): boolean {
    if (arr === prefix || !prefix.length) {
        return true;
    }

    const limit = arr.length;
    const preLimit = prefix.length;

    if (preLimit > limit) {
        return false;
    }

    for (let i = 0; i < preLimit; i++) {
        if (arr[i] !== prefix[i]) {
            return false;
        }
    }

    return true;
}

/** Checks if the array-like object ends with the given suffix. */
export function endsWith<T>(arr: RealArrayLike<T>, suffix: RealArrayLike<T>): boolean {
    if (arr === suffix || !suffix.length) {
        return true;
    }

    const limit = arr.length;
    const sufLimit = suffix.length;
    const offset = limit - sufLimit;

    if (offset < 0) {
        return false;
    }

    for (let i = 0; i < sufLimit; i++) {
        if (arr[offset + i] !== suffix[i]) {
            return false;
        }
    }

    return true;
}

/** Breaks the array-like object into smaller chunks according to the given delimiter. */
export function split<T>(arr: RealArrayLike<T>, delimiter: T): RealArrayLike<T>[] {
    const chunks: (typeof arr)[] = [];
    const limit = arr.length;
    let offset = 0;

    for (let i = 0; i < limit; i++) {
        if (arr[i] === delimiter) {
            chunks.push(arr.slice(offset, i));
            offset = i + 1;
        }
    }

    if (offset < limit) {
        chunks.push(arr.slice(offset, limit));
    } else if (offset === limit) {
        const ctor = arr.constructor as (new (...args: any[]) => RealArrayLike<T>) & {
            from?: (iterable: Iterable<T>) => RealArrayLike<T>;
        };

        if (typeof ctor.from === "function") {
            chunks.push(ctor.from([]));
        } else {
            chunks.push(new ctor([]));
        }
    }

    return chunks;
}

/** Breaks the array-like object into smaller chunks according to the given length. */
export function chunk<T>(arr: RealArrayLike<T>, length: number): RealArrayLike<T>[] {
    const limit = arr.length;
    const size = Math.ceil(limit / length);
    const chunks = new Array<RealArrayLike<T>>(size);
    let offset = 0;
    let idx = 0;

    while (offset < limit) {
        chunks[idx] = arr.slice(offset, offset + length);
        offset += length;
        idx++;
    }

    return chunks;
}
