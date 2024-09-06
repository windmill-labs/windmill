
import { BiMap, CiMap } from "../collections.ts";

declare global {
    class BiMap<K, V> extends Map<K, V> {
        getKey(value: V): K | undefined;
        hasValue(value: V): boolean;
        deleteValue(value: V): boolean;
    }

    class CiMap<K extends string, V> extends Map<K, any> {
        readonly [Symbol.toStringTag]: "CiMap";
        constructor(iterable?: Iterable<readonly [K, V]> | null);
        override set(key: K, value: V): this;
        override get(key: K): V | undefined;
        override has(key: K): boolean;
        override delete(key: K): boolean;
        override entries(): IterableIterator<[K, V]>;
        override keys(): IterableIterator<K>;
        override values(): IterableIterator<V>;
        override forEach(callbackfn: (value: V, key: K, map: Map<K, V>) => void, thisArg?: any): void;
        [Symbol.iterator](): IterableIterator<[K, V]>;
    }
}

// @ts-ignore
globalThis["BiMap"] = BiMap;
// @ts-ignore
globalThis["CiMap"] = CiMap;
