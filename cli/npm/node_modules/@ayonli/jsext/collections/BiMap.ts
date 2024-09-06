const inverse = Symbol("inverse");

/**
 * Bi-directional map, keys and values are unique and map to each other.
 * 
 * @example
 * ```ts
 * import { BiMap } from "@ayonli/jsext/collections";
 * 
 * const map = new BiMap<string, string>();
 * 
 * map.set("foo", "hello");
 * map.set("bar", "world");
 * 
 * console.log(map.get("foo")); // hello
 * console.log(map.getKey("world")); // bar
 * 
 * map.delete("foo");
 * console.log(map.hasValue("hello")); // false
 * 
 * map.deleteValue("world");
 * console.log(map.has("bar")); // false
 * ```
 */
export default class BiMap<K, V> extends Map<K, V> {
    [inverse]: Map<V, K>;

    get [Symbol.toStringTag](): "BiMap" {
        return "BiMap";
    }

    constructor(iterable: Iterable<readonly [K, V]> | null = null) {
        super();
        this[inverse] = new Map<V, K>();

        if (iterable) {
            for (const [key, value] of iterable) {
                this.set(key, value);
            }
        }
    }

    override set(key: K, value: V): this {
        super.set(key, value);
        this[inverse].set(value, key);
        return this;
    }

    getKey(value: V): K | undefined {
        return this[inverse].get(value);
    }

    hasValue(value: V): boolean {
        return this[inverse].has(value);
    }

    deleteValue(value: V): boolean {
        if (this[inverse].has(value)) {
            const key = this[inverse].get(value) as K;
            super.delete(key);
            this[inverse].delete(value);
            return true;
        }

        return false;
    }

    override clear(): void {
        super.clear();
        this[inverse].clear();
    }
}
