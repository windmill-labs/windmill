/**
 * Case-insensitive map, keys are case-insensitive.
 * 
 * @example
 * ```ts
 * import { CiMap } from "@ayonli/jsext/collections";
 * 
 * const map = new CiMap<string, string>();
 * 
 * map.set("foo", "hello");
 * map.set("bar", "world");
 * 
 * console.log(map.get("FOO")); // hello
 * console.log(map.has("BAR")); // true
 * ```
 */
export default class CiMap<K extends string, V> extends Map<K, any> {
    get [Symbol.toStringTag](): "CiMap" {
        return "CiMap";
    }

    constructor(iterable: Iterable<readonly [K, V]> | null = null) {
        super();

        if (iterable) {
            for (const [key, value] of iterable) {
                this.set(key, value);
            }
        }
    }

    override set(key: K, value: V): this {
        const id = String(key).toLowerCase();
        super.set(id as K, { key, value });
        return this;
    }

    override get(key: K): V | undefined {
        const id = String(key).toLowerCase();
        return super.get(id as K)?.value;
    }

    override has(key: K): boolean {
        const id = String(key).toLowerCase();
        return super.has(id as K);
    }

    override delete(key: K): boolean {
        const id = String(key).toLowerCase();
        return super.delete(id as K);
    }

    override * entries(): IterableIterator<[K, V]> {
        for (const { key, value } of super.values()) {
            yield [key, value];
        }
    }

    override * keys(): IterableIterator<K> {
        for (const { key } of super.values()) {
            yield key;
        }
    }

    override * values(): IterableIterator<V> {
        for (const { value } of super.values()) {
            yield value;
        }
    }

    override forEach(callbackfn: (value: V, key: K, map: Map<K, V>) => void, thisArg?: any): void {
        super.forEach(({ key, value }) => {
            callbackfn(value, key, this);
        }, thisArg);
    }

    [Symbol.iterator](): IterableIterator<[K, V]> {
        return this.entries();
    }
}
