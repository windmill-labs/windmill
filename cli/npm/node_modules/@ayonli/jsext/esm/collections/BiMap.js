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
class BiMap extends Map {
    get [Symbol.toStringTag]() {
        return "BiMap";
    }
    constructor(iterable = null) {
        super();
        this[inverse] = new Map();
        if (iterable) {
            for (const [key, value] of iterable) {
                this.set(key, value);
            }
        }
    }
    set(key, value) {
        super.set(key, value);
        this[inverse].set(value, key);
        return this;
    }
    getKey(value) {
        return this[inverse].get(value);
    }
    hasValue(value) {
        return this[inverse].has(value);
    }
    deleteValue(value) {
        if (this[inverse].has(value)) {
            const key = this[inverse].get(value);
            super.delete(key);
            this[inverse].delete(value);
            return true;
        }
        return false;
    }
    clear() {
        super.clear();
        this[inverse].clear();
    }
}

export { BiMap as default };
//# sourceMappingURL=BiMap.js.map
