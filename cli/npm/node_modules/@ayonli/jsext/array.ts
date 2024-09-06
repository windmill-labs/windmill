/**
 * Functions for dealing with arrays.
 * @module
 */

import { isSubclassOf } from "./class.ts";
import { random as rand } from "./number.ts";
import {
    count as _count,
    equals as _equals,
    includeSlice as _includesSlice,
    startsWith as _startsWith,
    endsWith as _endsWith,
    split as _split,
    chunk as _chunk,
} from "./array/base.ts";

/**
 * Returns the first element of the array, or `undefined` if the array is empty.
 * This function is equivalent to `arr[0]` or `arr.at(0)`.
 */
export function first<T>(arr: T[]): T | undefined {
    return arr[0];
}

/**
 * Returns the last element of the array, or `undefined` if the array is empty.
 * This function is equivalent to `arr[arr.length - 1]` or `arr.at(-1)`.
 */
export function last<T>(arr: T[]): T | undefined {
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
export function random<T>(arr: T[], remove = false): T | undefined {
    if (!arr.length) {
        return undefined;
    } else if (arr.length === 1) {
        if (remove) {
            return arr.splice(0, 1)[0];
        } else {
            return arr[0];
        }
    }

    const i = rand(0, arr.length - 1);

    if (remove) {
        return arr.splice(i, 1)[0];
    } else {
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
export function count<T>(arr: T[], item: T): number {
    return _count(arr, item);
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
export function equals<T>(arr1: T[], arr2: T[]): boolean {
    return _equals(arr1, arr2);
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
export function includesSlice<T>(arr: T[], slice: T[]): boolean {
    return _includesSlice(arr, slice);
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
export function startsWith<T>(arr: T[], prefix: T[]): boolean {
    return _startsWith(arr, prefix);
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
export function endsWith<T>(arr: T[], suffix: T[]): boolean {
    return _endsWith(arr, suffix);
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
export function split<T>(arr: T[], delimiter: T): T[][] {
    return _split(arr, delimiter) as T[][];
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
export function chunk<T>(arr: T[], length: number): T[][] {
    return _chunk(arr, length) as T[][];
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
export function unique<T>(arr: T[]): T[] {
    return [...new Set(arr)];
}

/**
 * @deprecated use `unique` instead.
 */
export const uniq = unique;

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
export function uniqueBy<T, K extends string | number | symbol>(
    arr: T[],
    fn: (item: T, i: number) => K
): T[] {
    const map = new Map() as Map<K, T>;

    for (let i = 0; i < arr.length; i++) {
        const item = arr[i] as T;
        const key = fn(item, i);
        map.has(key) || map.set(key, item);
    }

    return [...map.values()];
}

/**
 * @deprecated use `uniqueBy` instead.
 */
export const uniqBy = uniqueBy;

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
export function shuffle<T>(arr: T[]): T[] {
    for (let i = arr.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [arr[i], arr[j]] = [arr[j] as T, arr[i] as T];
    }

    return arr;
}

/**
 * Orders the items of the array according to the given callback function.
 * 
 * @example
 * ```ts
 * import { orderBy } from "@ayonli/jsext/array";
 * 
 * const arr = [
 *     { id: 1, name: "foo" },
 *     { id: 2, name: "bar" },
 *     { id: 3, name: "baz" },
 *     { id: 4, name: "qux" },
 * ];
 * 
 * console.log(orderBy(arr, item => item.name));
 * // [
 * //     { id: 2, name: "bar" },
 * //     { id: 3, name: "baz" },
 * //     { id: 1, name: "foo" },
 * //     { id: 4, name: "qux" }
 * // ]
 * 
 * console.log(orderBy(arr, item => item.id, "desc"));
 * // [
 * //     { id: 4, name: "qux" },
 * //     { id: 3, name: "baz" },
 * //     { id: 2, name: "bar" },
 * //     { id: 1, name: "foo" }
 * // ]
 * ```
 */
export function orderBy<T>(
    arr: T[],
    fn: (item: T, i: number) => string | number | bigint,
    order?: "asc" | "desc"
): T[];
/**
 * Orders the items of the array according to the specified comparable `key`
 * (whose value must either be a numeric or string).
 * 
 * @deprecated This signature is not in line with other functions, such as
 * {@link groupBy} and {@link keyBy}, use the callback form instead.
 */
export function orderBy<T>(arr: T[], key: keyof T, order?: "asc" | "desc"): T[];
export function orderBy<T>(
    arr: T[],
    key: ((item: T, i: number) => string | number | bigint) | keyof T,
    order: "asc" | "desc" = "asc"
): T[] {
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
            Array.isArray(a) || Array.isArray(b)
        ) {
            return -1;
        }

        const _a = a[key];
        const _b = b[key];

        if (_a === undefined || _b === undefined) {
            return -1;
        }

        if (typeof _a === "number" && typeof _b === "number") {
            return _a - _b;
        } else if ((typeof _a === "string" && typeof _b === "string")
            || (typeof _a === "bigint" && typeof _b === "bigint")
        ) {
            if (_a < _b) {
                return -1;
            } else if (_a > _b) {
                return 1;
            } else {
                return 1;
            }
        } else {
            return -1;
        }
    });

    if (order === "desc") {
        items.reverse();
    }

    return items;
};

/**
 * Groups the items of the array according to the comparable values returned by a provided
 * callback function.
 * 
 * The returned record / map has separate properties for each group, containing arrays with
 * the items in the group.
 * 
 * @example
 * ```ts
 * import { groupBy } from "@ayonli/jsext/array";
 * 
 * const arr = [
 *     { id: 1, name: "foo" },
 *     { id: 2, name: "bar" },
 *     { id: 3, name: "foo" },
 *     { id: 4, name: "baz" },
 *     { id: 5, name: "bar" },
 * ];
 * 
 * console.log(groupBy(arr, item => item.name));
 * // {
 * //     foo: [
 * //         { id: 1, name: "foo" },
 * //         { id: 3, name: "foo" }
 * //     ],
 * //     bar: [
 * //         { id: 2, name: "bar" },
 * //         { id: 5, name: "bar" }
 * //     ],
 * //     baz: [
 * //         { id: 4, name: "baz" }
 * //     ]
 * // }
 * ```
 */
export function groupBy<T, K extends string | number | symbol>(
    arr: T[],
    fn: (item: T, i: number) => K,
    type?: ObjectConstructor
): Record<K, T[]>;
/**
 * @example
 * ```ts
 * import { groupBy } from "@ayonli/jsext/array";
 * 
 * const arr = [
 *     { id: 1, name: "foo" },
 *     { id: 2, name: "bar" },
 *     { id: 3, name: "foo" },
 *     { id: 4, name: "baz" },
 *     { id: 5, name: "bar" },
 * ];
 * 
 * console.log(groupBy(arr, item => item.name, Map));
 * // Map {
 * //     "foo" => [
 * //         { id: 1, name: "foo" },
 * //         { id: 3, name: "foo" }
 * //     ],
 * //     "bar" => [
 * //         { id: 2, name: "bar" },
 * //         { id: 5, name: "bar" }
 * //     ],
 * //     "baz" => [
 * //         { id: 4, name: "baz" }
 * //     ]
 * // }
 * ```
 */
export function groupBy<T, K>(
    arr: T[],
    fn: (item: T, i: number) => K,
    type: MapConstructor
): Map<K, T[]>;
export function groupBy<T, K>(
    arr: T[],
    fn: (item: T, i: number) => K,
    type: ObjectConstructor | MapConstructor = Object
): any {
    if (type === Map || isSubclassOf(type, Map)) {
        const groups = new type() as Map<any, T[]>;

        for (let i = 0; i < arr.length; i++) {
            const item = arr[i] as T;
            const key = fn(item, i);
            const list = groups.get(key);

            if (list) {
                list.push(item);
            } else {
                groups.set(key, [item]);
            }
        }

        return groups;
    } else {
        const groups: Record<string | number | symbol, T[]> = {};

        for (let i = 0; i < arr.length; i++) {
            const item = arr[i] as T;
            const key = fn(item, i) as string | number | symbol;
            const list = groups[key];

            if (list) {
                list.push(item);
            } else {
                groups[key] = [item];
            }
        }

        return groups;
    }
};

/**
 * Creates a record or map from the items of the array according to the comparable values
 * returned by a provided callback function.
 * 
 * This function is similar to {@link groupBy}, except it overrides values if the same
 * property already exists instead of grouping them as a list.
 * 
 * @example
 * ```ts
 * import { keyBy } from "@ayonli/jsext/array";
 * 
 * const arr = [
 *     { id: 1, name: "foo" },
 *     { id: 2, name: "bar" },
 *     { id: 3, name: "baz" },
 * ];
 * 
 * console.log(keyBy(arr, item => item.name));
 * // {
 * //     foo: { id: 1, name: "foo" },
 * //     bar: { id: 2, name: "bar" },
 * //     baz: { id: 3, name: "baz" }
 * // }
 * ```
 */
export function keyBy<T, K extends string | number | symbol>(
    arr: T[],
    fn: (item: T, i: number) => K,
    type?: ObjectConstructor
): Record<K, T>;
/**
 * @example
 * ```ts
 * import { keyBy } from "@ayonli/jsext/array";
 * 
 * const arr = [
 *     { id: 1, name: "foo" },
 *     { id: 2, name: "bar" },
 *     { id: 3, name: "baz" },
 * ];
 * 
 * console.log(keyBy(arr, item => item.name, Map));
 * // Map {
 * //     "foo" => { id: 1, name: "foo" },
 * //     "bar" => { id: 2, name: "bar" },
 * //     "baz" => { id: 3, name: "baz" }
 * // }
 * ```
 */
export function keyBy<T, K>(
    arr: T[],
    fn: (item: T, i: number) => K,
    type: MapConstructor
): Map<K, T>;
export function keyBy<T, K>(
    arr: T[],
    fn: (item: T, i: number) => K,
    type: ObjectConstructor | MapConstructor = Object
): Record<string | number | symbol, T> | Map<K, T> {
    if (type === Map || isSubclassOf(type, Map)) {
        const map = new type() as Map<any, T>;

        for (let i = 0; i < arr.length; i++) {
            const item = arr[i] as T;
            const key = fn(item, i);
            map.set(key, item);
        }

        return map;
    } else {
        const record = {} as Record<string | number | symbol, T>;

        for (let i = 0; i < arr.length; i++) {
            const item = arr[i] as T;
            const key = fn(item, i) as string | number | symbol;
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
export function partition<T>(
    arr: T[],
    predicate: (item: T, i: number) => boolean
): [T[], T[]] {
    const match: T[] = [];
    const rest: T[] = [];

    for (let i = 0; i < arr.length; i++) {
        const item = arr[i] as T;
        (predicate(item, i) ? match : rest).push(item);
    }

    return [match, rest];
}
