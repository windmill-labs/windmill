import {
    first as _first,
    last as _last,
    chunk as _chunk,
    count as _count,
    equals as _equals,
    includesSlice as _includesSlice,
    startsWith as _startsWith,
    endsWith as _endsWith,
    groupBy as _groupBy,
    keyBy as _keyBy,
    orderBy as _orderBy,
    random as _random,
    shuffle as _shuffle,
    split as _split,
    unique as _unique,
    uniqueBy as _uniqueBy,
    partition as _partition,
} from "../array.ts";

declare global {
    interface Array<T> {
        /** Returns the first element of the array, or `undefined` if the array is empty. */
        first(): T | undefined;
        /** Returns the last element of the array, or `undefined` if the array is empty. */
        last(): T | undefined;
        /** Returns a random element of the array, or `undefined` if the array is empty. */
        random(remove?: boolean): T | undefined;
        /** Counts the occurrence of the element in the array. */
        count(item: T): number;
        /**
         * Performs a shallow compare to another array and see if it contains the same elements as
         * this array.
         */
        equals(another: T[]): boolean;
        /** Checks if the array contains another array as a slice of its contents. */
        includesSlice<T>(slice: T[]): boolean;
        /** Checks if the array starts with the given prefix. */
        startsWith<T>(prefix: T[]): boolean;
        /** Checks if the array ends with the given suffix. */
        endsWith<T>(suffix: T[]): boolean;
        /** Breaks the array into smaller chunks according to the given delimiter. */
        split(delimiter: T): T[][];
        /** Breaks the array into smaller chunks according to the given length. */
        chunk(length: number): T[][];
        /** Returns a subset of the array that contains only unique items. */
        unique(): T[];
        /**
         * @deprecated use `unique` instead.
         */
        uniq(): T[];
        /**
         * Returns a subset of the array that contains only unique items filtered by the
         * given callback function.
         */
        uniqueBy<K extends string | number | symbol>(fn: (item: T, i: number) => K): T[];
        /**
         * @deprecated use `uniqueBy` instead.
         */
        uniqBy<K extends string | number | symbol>(fn: (item: T, i: number) => K): T[];
        /**
         * Reorganizes the elements in the array in random order.
         * 
         * This function mutates the array.
         */
        shuffle(): T[];
        toShuffled(): T[];
        toReversed(): T[];
        toSorted(fn?: ((a: T, b: T) => number) | undefined): T[];
        /** Orders the items of the array according to the given callback function. */
        orderBy(fn: (item: T, i: number) => string | number | bigint, order?: "asc" | "desc"): T[];
        /**
         * Orders the items of the array according to the specified comparable `key`
         * (whose value must either be a numeric or string).
         * 
         * @deprecated This signature is not in line with other functions, such as
         * {@link groupBy} and {@link keyBy}, use the callback form instead.
         */
        orderBy(key: keyof T, order?: "asc" | "desc"): T[];
        /**
         * Groups the items of the array according to the comparable values returned by a provided
         * callback function.
         * 
         * The returned record / map has separate properties for each group, containing arrays with
         * the items in the group.
         */
        groupBy<K extends string | number | symbol>(fn: (item: T, i: number) => K, type?: ObjectConstructor): Record<K, T[]>;
        groupBy<K>(fn: (item: T, i: number) => K, type: MapConstructor): Map<K, T[]>;
        /**
         * Creates a record or map from the items of the array according to the comparable values
         * returned by a provided callback function.
         * 
         * This function is similar to {@link groupBy} except it overrides values if the same
         * property already exists instead of grouping them as a list.
         */
        keyBy<K extends string | number | symbol>(fn: (item: T, i: number) => K, type?: ObjectConstructor): Record<K, T>;
        keyBy<K>(fn: (item: T, i: number) => K, type: MapConstructor): Map<K, T>;
        /**
         * Returns a tuple of two arrays with the first one containing all elements in
         * the given array that match the given predicate and the second one containing
         * all that do not.
         */
        partition(predicate: (item: T, i: number) => boolean): [T[], T[]];
    }
}

Array.prototype.first = function first() {
    return _first(this);
};

Array.prototype.last = function last() {
    return _last(this);
};

Array.prototype.random = function random(remove = false) {
    return _random(this, remove);
};

Array.prototype.count = function count(ele) {
    return _count(this, ele);
};

Array.prototype.equals = function equals(another) {
    return _equals(this, another);
};

Array.prototype.includesSlice = function includesSlice(slice) {
    return _includesSlice(this, slice);
};

Array.prototype.startsWith = function startsWith(prefix) {
    return _startsWith(this, prefix);
};

Array.prototype.endsWith = function endsWith(suffix) {
    return _endsWith(this, suffix);
};

Array.prototype.split = function split(delimiter) {
    return _split(this, delimiter);
};

Array.prototype.chunk = function chunk(length) {
    return _chunk(this, length);
};

Array.prototype.unique = Array.prototype.uniq = function unique() {
    return _unique(this);
};

Array.prototype.uniqueBy = Array.prototype.uniqBy = function uniqueBy(fn) {
    return _uniqueBy(this, fn);
};

Array.prototype.shuffle = function shuffle() {
    return _shuffle(this);
};

Array.prototype.toShuffled = function toShuffled() {
    return this.slice().shuffle();
};

if (!Array.prototype.toReversed) {
    Array.prototype.toReversed = function toReversed() {
        return this.slice().reverse();
    };
}

if (!Array.prototype.toSorted) {
    Array.prototype.toSorted = function toSorted(fn) {
        return this.slice().sort(fn);
    };
}

Array.prototype.orderBy = function orderBy(key, order = "asc") {
    return _orderBy(this, key as any, order);
};

Array.prototype.groupBy = function groupBy(
    fn: (item: any, i: number) => any,
    type: ObjectConstructor | MapConstructor = Object
): any {
    return _groupBy(this, fn, type as any);
};

Array.prototype.keyBy = function keyBy(
    fn: (item: any, i: number) => any,
    type: ObjectConstructor | MapConstructor = Object
): any {
    return _keyBy(this, fn, type as any);
};

Array.prototype.partition = function partition(predicate) {
    return _partition(this, predicate);
};
