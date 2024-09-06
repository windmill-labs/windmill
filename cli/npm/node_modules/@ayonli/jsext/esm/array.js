import { isSubclassOf } from './class.js';
import { random as random$1 } from './number.js';
import { count as count$1, equals as equals$1, includeSlice, startsWith as startsWith$1, endsWith as endsWith$1, split as split$1, chunk as chunk$1 } from './array/base.js';

/**
 * Functions for dealing with arrays.
 * @module
 */
/**
 * Returns the first element of the array, or `undefined` if the array is empty.
 * This function is equivalent to `arr[0]` or `arr.at(0)`.
 */
function first(arr) {
    return arr[0];
}
/**
 * Returns the last element of the array, or `undefined` if the array is empty.
 * This function is equivalent to `arr[arr.length - 1]` or `arr.at(-1)`.
 */
function last(arr) {
    return arr.length > 0 ? arr[arr.length - 1] : undefined;
}
/**
 * Returns a random element of the array, or `undefined` if the array is empty.
 * @param remove If `true`, the element will be removed from the array.
 *
 * @example
 * ```ts
 * import { random } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5];
 *
 * console.log(random(arr)); // 3 for example
 *
 * console.log(random(arr, true)); // 3 for example
 * console.log(arr); // [1, 2, 4, 5]
 * ```
 */
function random(arr, remove = false) {
    if (!arr.length) {
        return undefined;
    }
    else if (arr.length === 1) {
        if (remove) {
            return arr.splice(0, 1)[0];
        }
        else {
            return arr[0];
        }
    }
    const i = random$1(0, arr.length - 1);
    if (remove) {
        return arr.splice(i, 1)[0];
    }
    else {
        return arr[i];
    }
}
/**
 * Counts the occurrence of the element in the array.
 *
 * @example
 * ```ts
 * import { count } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5, 2, 3, 4, 2];
 *
 * console.log(count(arr, 2)); // 3
 * console.log(count(arr, 6)); // 0
 * ```
 */
function count(arr, item) {
    return count$1(arr, item);
}
/**
 * Performs a shallow compare to another array and see if it contains the same elements as
 * this array.
 *
 * @example
 * ```ts
 * import { equals } from "@ayonli/jsext/array";
 *
 * const arr1 = [1, 2, 3, 4, 5];
 * const arr2 = [{ foo: "bar" }];
 *
 * console.log(equals(arr1, [1, 2, 3, 4, 5])); // true
 * console.log(equals(arr2, [{ foo: "bar" }])); // false, object refs are different
 * ```
 */
function equals(arr1, arr2) {
    return equals$1(arr1, arr2);
}
/**
 * Checks if the array contains another array as a slice of its contents.
 *
 * @example
 * ```ts
 * import { includesSlice } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5];
 *
 * console.log(includesSlice(arr, [2, 3, 4])); // true
 * console.log(includesSlice(arr, [2, 4, 3])); // false
 * ```
 */
function includesSlice(arr, slice) {
    return includeSlice(arr, slice);
}
/**
 * Checks if the array starts with the given prefix.
 *
 * @example
 * ```ts
 * import { startsWith } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5];
 *
 * console.log(startsWith(arr, [1, 2])); // true
 * console.log(startsWith(arr, [2, 1])); // false
 * ```
 */
function startsWith(arr, prefix) {
    return startsWith$1(arr, prefix);
}
/**
 * Checks if the array ends with the given suffix.
 *
 * @example
 * ```ts
 * import { endsWith } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5];
 *
 * console.log(endsWith(arr, [4, 5])); // true
 * console.log(endsWith(arr, [5, 4])); // false
 * ```
 */
function endsWith(arr, suffix) {
    return endsWith$1(arr, suffix);
}
/**
 * Breaks the array into smaller chunks according to the given delimiter.
 *
 * @example
 * ```ts
 * import { split } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5, 6, 7, 8, 9];
 *
 * console.log(split(arr, 5)); // [[1, 2, 3, 4], [6, 7, 8, 9]]
 * ```
 */
function split(arr, delimiter) {
    return split$1(arr, delimiter);
}
/**
 * Breaks the array into smaller chunks according to the given length.
 *
 * @example
 * ```ts
 * import { chunk } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5, 6, 7, 8, 9];
 *
 * console.log(chunk(arr, 5)); // [[1, 2, 3, 4, 5], [6, 7, 8, 9]]
 * ```
 */
function chunk(arr, length) {
    return chunk$1(arr, length);
}
/**
 * Returns a subset of the array that contains only unique items.
 *
 * @example
 * ```ts
 * import { unique } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5, 2, 3, 4, 2];
 *
 * console.log(unique(arr)); // [1, 2, 3, 4, 5]
 * ```
 */
function unique(arr) {
    return [...new Set(arr)];
}
/**
 * @deprecated use `unique` instead.
 */
const uniq = unique;
/**
 * Returns a subset of the array that contains only unique items filtered by the
 * given callback function.
 *
 * @example
 * ```ts
 * import { uniqueBy } from "@ayonli/jsext/array";
 *
 * const arr = [
 *     { id: 1, name: "foo" },
 *     { id: 2, name: "bar" },
 *     { id: 3, name: "foo" },
 *     { id: 4, name: "baz" },
 *     { id: 5, name: "bar" },
 * ];
 *
 * console.log(uniqueBy(arr, item => item.name));
 * // [
 * //     { id: 1, name: "foo" },
 * //     { id: 2, name: "bar" },
 * //     { id: 4, name: "baz" }
 * // ]
 * ```
 */
function uniqueBy(arr, fn) {
    const map = new Map();
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        const key = fn(item, i);
        map.has(key) || map.set(key, item);
    }
    return [...map.values()];
}
/**
 * @deprecated use `uniqueBy` instead.
 */
const uniqBy = uniqueBy;
/**
 * Reorganizes the elements in the array in random order.
 *
 * This function mutates the array.
 *
 * @example
 * ```ts
 * import { shuffle } from "@ayonli/jsext/array";
 *
 * const arr = [1, 2, 3, 4, 5];
 *
 * console.log(shuffle(arr)); // [3, 1, 5, 2, 4] for example
 * console.log(arr); // [3, 1, 5, 2, 4]
 * ```
 */
function shuffle(arr) {
    for (let i = arr.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [arr[i], arr[j]] = [arr[j], arr[i]];
    }
    return arr;
}
function orderBy(arr, key, order = "asc") {
    const items = arr.slice();
    if (typeof key === "function") {
        return orderBy(items.map((item, i) => ({
            key: key(item, i),
            value: item,
        })), "key", order).map(({ value }) => value);
    }
    items.sort((a, b) => {
        if (typeof a !== "object" || typeof b !== "object" ||
            !a || !b ||
            Array.isArray(a) || Array.isArray(b)) {
            return -1;
        }
        const _a = a[key];
        const _b = b[key];
        if (_a === undefined || _b === undefined) {
            return -1;
        }
        if (typeof _a === "number" && typeof _b === "number") {
            return _a - _b;
        }
        else if ((typeof _a === "string" && typeof _b === "string")
            || (typeof _a === "bigint" && typeof _b === "bigint")) {
            if (_a < _b) {
                return -1;
            }
            else if (_a > _b) {
                return 1;
            }
            else {
                return 1;
            }
        }
        else {
            return -1;
        }
    });
    if (order === "desc") {
        items.reverse();
    }
    return items;
}
function groupBy(arr, fn, type = Object) {
    if (type === Map || isSubclassOf(type, Map)) {
        const groups = new type();
        for (let i = 0; i < arr.length; i++) {
            const item = arr[i];
            const key = fn(item, i);
            const list = groups.get(key);
            if (list) {
                list.push(item);
            }
            else {
                groups.set(key, [item]);
            }
        }
        return groups;
    }
    else {
        const groups = {};
        for (let i = 0; i < arr.length; i++) {
            const item = arr[i];
            const key = fn(item, i);
            const list = groups[key];
            if (list) {
                list.push(item);
            }
            else {
                groups[key] = [item];
            }
        }
        return groups;
    }
}
function keyBy(arr, fn, type = Object) {
    if (type === Map || isSubclassOf(type, Map)) {
        const map = new type();
        for (let i = 0; i < arr.length; i++) {
            const item = arr[i];
            const key = fn(item, i);
            map.set(key, item);
        }
        return map;
    }
    else {
        const record = {};
        for (let i = 0; i < arr.length; i++) {
            const item = arr[i];
            const key = fn(item, i);
            record[key] = item;
        }
        return record;
    }
}
/**
 * Returns a tuple of two arrays with the first one containing all elements in
 * the given array that match the given predicate and the second one containing
 * all that do not.
 *
 * @example
 * ```ts
 * import { partition } from "@ayonli/jsext/array";
 *
 * const arr = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
 * const [even, odd] = partition(arr, item => item % 2 === 0);
 *
 * console.log(even); // [0, 2, 4, 6, 8]
 * console.log(odd); // [1, 3, 5, 7, 9]
 * ```
 */
function partition(arr, predicate) {
    const match = [];
    const rest = [];
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        (predicate(item, i) ? match : rest).push(item);
    }
    return [match, rest];
}

export { chunk, count, endsWith, equals, first, groupBy, includesSlice, keyBy, last, orderBy, partition, random, shuffle, split, startsWith, uniq, uniqBy, unique, uniqueBy };
//# sourceMappingURL=array.js.map
