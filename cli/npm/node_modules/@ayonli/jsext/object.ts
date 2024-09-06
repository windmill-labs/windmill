/**
 * Functions for dealing with objects.
 * @module
 */

import { Constructor } from "./types.ts";
import { isClass } from "./class.ts";

/**
 * Returns `true` if the specified object has the indicated property as its own property.
 * If the property is inherited, or does not exist, the function returns `false`.
 * 
 * @example
 * ```ts
 * import { hasOwn } from "@ayonli/jsext/object";
 * 
 * const obj = { foo: "hello" };
 * 
 * console.log(hasOwn(obj, "foo")); // true
 * console.log(hasOwn(obj, "toString")); // false
 * ```
 */
export function hasOwn(obj: any, key: string | number | symbol): boolean {
    return Object.prototype.hasOwnProperty.call(obj, key);
}

/**
 * Returns `true` if the specified object has the indicated method as its own method (in its own
 * prototype). If the method is inherited, or is not in the prototype, or does not exist, this
 * function returns `false`.
 * 
 * @example
 * ```ts
 * import { hasOwnMethod } from "@ayonli/jsext/object";
 * 
 * class MyClass {
 *     foo() {
 *         return "Hello";
 *     }
 * 
 *     bar = () => "World";
 * }
 * 
 * const obj = new MyClass();
 * 
 * console.log(hasOwnMethod(obj, "foo")); // true
 * console.log(hasOwnMethod(obj, "bar")); // false
 * console.log(hasOwnMethod(obj, "toString")); // false
 * ```
 */
export function hasOwnMethod(obj: any, method: string | symbol): boolean {
    const proto = Object.getPrototypeOf(obj);

    if (!proto || !hasOwn(proto, method)) {
        return false;
    }

    return typeof Object.getOwnPropertyDescriptor(proto, method)?.value === "function";
}

/**
 * Copies the key-value pairs that are presented in the source objects but are missing in
 * the target object into the target, later pairs are skipped if the same key already exists.
 * 
 * This function mutates the target object and returns it.
 * 
 * @example
 * ```ts
 * import { patch } from "@ayonli/jsext/object";
 * 
 * const obj1 = { foo: "Hello" };
 * const obj2 = { foo: "Hi", bar: "World" };
 * 
 * console.log(patch(obj1, obj2)); // { foo: "Hello", bar: "World" }
 * ```
 */
export function patch<T extends {}, U>(target: T, source: U): T & U;
export function patch<T extends {}, U, V>(target: T, source1: U, source2: V): T & U & V;
export function patch<T extends {}, U, V, W>(target: T, source1: U, source2: V, source3: W): T & U & V & W;
export function patch(target: object, ...sources: any[]): any;
export function patch(target: any, ...sources: any[]) {
    for (const source of sources) {
        for (const key of Reflect.ownKeys(source)) {
            if (!hasOwn(target, key) || target[key] === undefined) {
                target[key] = source[key];
            }
        }
    }

    return target;
}

/**
 * Creates an object composed of the picked keys.
 * 
 * @example
 * ```ts
 * import { pick } from "@ayonli/jsext/object";
 * 
 * const obj = { foo: "Hello", bar: "World" };
 * 
 * console.log(pick(obj, ["foo"])); // { foo: "Hello" }
 * ```
 */
export function pick<T extends object, U extends keyof T>(obj: T, keys: U[]): Pick<T, U>;
export function pick<T>(obj: T, keys: (string | symbol)[]): Partial<T>;
export function pick(obj: any, keys: (string | symbol)[]) {
    return keys.reduce((result: any, key: string | symbol) => {
        if (key in obj && obj[key] !== undefined) {
            result[key] = obj[key];
        }

        return result;
    }, {});
}

/**
 * Creates an object composed without the picked keys.
 * 
 * **NOTE:**
 * This function only collect keys from the object's own properties, except for type Error,
 * whose `name`, `message` and `cause` are always collected.
 * 
 * @example
 * ```ts
 * import { omit } from "@ayonli/jsext/object";
 * 
 * const obj = { foo: "Hello", bar: "World" };
 * 
 * console.log(omit(obj, ["foo"])); // { bar: "World" }
 * ```
 */
export function omit<T extends object, U extends keyof T>(obj: T, keys: U[]): Omit<T, U>;
export function omit<T>(obj: T, keys: (string | symbol)[]): Partial<T>;
export function omit(obj: any, keys: (string | symbol)[]) {
    const allKeys = Reflect.ownKeys(obj);
    const keptKeys = allKeys.filter(key => !keys.includes(key));
    const result = pick(obj, keptKeys);

    // special treatment for Error types
    if (obj instanceof Error) {
        ["name", "message", "stack", "cause"].forEach(key => {
            if (!keys.includes(key) &&
                (obj as any)[key] !== undefined &&
                !hasOwn(result, key)
            ) {
                result[key] = (obj as any)[key];
            }
        });
    }

    return result;
}

/**
 * Checks if the value is an instance of the given type, returns the value itself if passed,
 * otherwise returns `null`. This function is mainly used for the optional chaining syntax.
 * 
 * @example
 * ```ts
 * import { as } from "@ayonli/jsext/object";
 * 
 * try {
 *     // ... do something
 * } catch (err) {
 *     console.error(as(err, Error)?.message ?? String(err));
 * }
 * ```
 */
export function as(value: unknown, type: StringConstructor): string | null;
export function as(value: unknown, type: NumberConstructor): number | null;
export function as(value: unknown, type: BigIntConstructor): bigint | null;
export function as(value: unknown, type: BooleanConstructor): boolean | null;
export function as(value: unknown, type: SymbolConstructor): symbol | null;
export function as<T>(value: unknown, type: Constructor<T>): T | null;
export function as(value: any, type: any): any {
    if (typeof type !== "function") {
        throw new TypeError("type must be a valid constructor");
    }

    let _type: any;
    const primitiveMap = <Record<string, Function>>{
        "string": String,
        "number": Number,
        "bigint": BigInt,
        "boolean": Boolean,
        "symbol": Symbol
    };

    if (value instanceof type) {
        if ([String, Number, Boolean].includes(type)) {
            return value.valueOf(); // make sure the primitives are returned.
        } else {
            return value;
        }
    } else if ((_type = typeof value) && primitiveMap[_type] === type) {
        return value;
    }

    return null;
}

export type TypeNames = "string"
    | "number"
    | "bigint"
    | "boolean"
    | "symbol"
    | "function"
    | "class"
    | "undefined"
    | "null";

/**
 * Returns a string representation or the constructor of the value's type.
 * 
 * **NOTE:** This function returns `"class"` for ES6 classes.
 * 
 * **NOTE:** This function returns `"null"` for `null`.
 * 
 * **NOTE:** This function returns `Object` for `Object.create(null)`.
 * 
 * @example
 * ```ts
 * import { typeOf } from "@ayonli/jsext/object";
 * 
 * console.log(typeOf("Hello")); // string
 * console.log(typeOf(42)); // number
 * console.log(typeOf(42n)); // bigint
 * console.log(typeOf(true)); // boolean
 * console.log(typeOf(Symbol("foo"))); // symbol
 * console.log(typeOf(() => {})); // function
 * console.log(typeOf(class Foo {})); // class
 * console.log(typeOf(undefined)); // undefined
 * console.log(typeOf(null)); // null
 * console.log(typeOf({ foo: "bar" })); // [Function: Object]
 * console.log(typeOf(Object.create(null))); // [Function: Object]
 * console.log(typeOf([1, 2, 3])); // [Function: Array]
 * console.log(typeOf(new Date())); // [Function: Date]
 * ```
 */
export function typeOf<T>(value: T): TypeNames | Constructor<T> {
    if (value === undefined) {
        return "undefined";
    } else if (value === null) {
        return "null";
    }

    const type = typeof value;

    if (type === "function") {
        return isClass(value) ? "class" : "function";
    } else if (type === "object") {
        return Object.getPrototypeOf(value)?.constructor as Constructor<T> ?? Object;
    } else {
        return type;
    }
}

/**
 * Returns `true` if the given value is valid. The following values are considered invalid:
 * 
 * - `undefined`
 * - `null`
 * - `NaN`
 * - `Invalid Date`
 */
export function isValid(value: unknown): boolean {
    return value !== undefined
        && value !== null
        && !Object.is(value, NaN)
        && !(value instanceof Date && value.toString() === "Invalid Date");
}

/**
 * Returns `true` is the given value is a plain object, that is, an object created by
 * the `Object` constructor or one with a `[[Prototype]]` of `null`.
 * 
 * @example
 * ```ts
 * import { isPlainObject } from "@ayonli/jsext/object";
 * 
 * console.log(isPlainObject({ foo: "bar" })); // true
 * console.log(isPlainObject(Object.create(null))); // true
 * console.log(isPlainObject(new Map([["foo", "bar"]]))); // false
 * ```
 */
export function isPlainObject(value: unknown): value is { [x: string | symbol]: any; } {
    if (typeof value !== "object" || value === null)
        return false;

    const proto = Object.getPrototypeOf(value);
    return proto === null || proto.constructor === Object;
}

/**
 * Creates an object base on the original object but without any invalid values
 * (except for `null`), and trims the value if it's a string.
 * 
 * **NOTE:** This function only operates on plain objects and arrays.
 * 
 * @example
 * ```ts
 * import { sanitize } from "@ayonli/jsext/object";
 * 
 * const obj = sanitize({
 *     foo: "Hello",
 *     bar: "  World  ",
 *     baz: undefined,
 *     num: NaN,
 * });
 * 
 * console.log(obj); // { foo: "Hello", bar: "World" }
 * ```
 */
export function sanitize<T extends object>(obj: T, deep = false, options: {
    removeNulls?: boolean;
    removeEmptyStrings?: boolean;
    removeEmptyObjects?: boolean;
    removeArrayItems?: boolean;
} = {}): T {
    const { removeNulls, removeEmptyStrings, removeEmptyObjects, removeArrayItems } = options;

    return (function process(target: any, depth: number): any {
        if (typeof target === "string") {
            return target.trim();
        } else if (Array.isArray(target)) {
            const arr = !depth || deep ? target.map(item => process(item, depth + 1)) : target;

            if (removeArrayItems) {
                return arr.filter(value => {
                    if (value === null) {
                        return !removeNulls;
                    } else if (value === "") {
                        return !removeEmptyStrings;
                    } else if (isValid(value)) {
                        if (typeof value !== "object") {
                            return true;
                        } else if (Array.isArray(value)) {
                            return value.length > 0 || !removeEmptyObjects;
                        } else if (isPlainObject(value)) {
                            return Reflect.ownKeys(value).length > 0 || !removeEmptyObjects;
                        } else {
                            return true;
                        }
                    } else {
                        return false;
                    }
                });
            } else {
                return arr;
            }
        } else if (isPlainObject(target)) {
            return !depth || deep ? Reflect.ownKeys(target).reduce((result, key) => {
                const value = process(target[key], depth + 1);

                if (value === null) {
                    if (!removeNulls) {
                        result[key] = value;
                    }
                } else if (value === "") {
                    if (!removeEmptyStrings) {
                        result[key] = value;
                    }
                } else if (isValid(value)) {
                    if (typeof value !== "object") {
                        result[key] = value;
                    } else if (Array.isArray(value)) {
                        if (value.length > 0 || !removeEmptyObjects) {
                            result[key] = value;
                        }
                    } else if (isPlainObject(value)) {
                        if (Reflect.ownKeys(value).length > 0 || !removeEmptyObjects) {
                            result[key] = value;
                        }
                    } else {
                        result[key] = value;
                    }
                }

                return result;
            }, target.constructor ? {} as any : Object.create(null)) : target;
        } else {
            return target;
        }
    })(obj, 0);
}

/**
 * Creates an object with sorted keys (in ascending order) of the original object.
 * 
 * **NOTE:** Symbol keys are not sorted and remain their original order.
 * 
 * **NOTE:** This function only operates on plain objects and arrays.
 * 
 * @example
 * ```ts
 * import { sortKeys } from "@ayonli/jsext/object";
 * 
 * const obj = sortKeys({ foo: "Hello", bar: "World" });
 * 
 * console.log(JSON.stringify(obj)); // { "bar": "World", "foo": "Hello" }
 * ```
 */
export function sortKeys<T extends object>(obj: T, deep = false): T {
    return (function process(target: any, depth: number): any {
        if (isPlainObject(target)) {
            return !depth || deep ? [
                ...Object.getOwnPropertyNames(target).sort(), // sort the string keys
                ...Object.getOwnPropertySymbols(target)
            ].reduce((result, key) => {
                result[key] = process(target[key], depth + 1);
                return result;
            }, target.constructor ? {} as any : Object.create(null)) : target;
        } else if (Array.isArray(target)) {
            return !depth || deep ? target.map(item => process(item, depth + 1)) : target;
        } else {
            return target;
        }
    })(obj, 0);
}

export type OmitChildrenNodes<T extends object> = Pick<T, {
    [K in keyof T]: T[K] extends any[] ? K : T[K] extends object ? never : K;
}[keyof T]>;

/**
 * Create an object with flatted keys of the original object, the children
 * nodes' properties will be transformed to a string-represented path.
 * 
 * **NOTE:** This function only operates on plain objects and arrays.
 * 
 * @param depth Default value: `1`.
 * @example
 * ```ts
 * import { flatKeys } from "@ayonli/jsext/object";
 * 
 * const obj = flatKeys({ foo: { bar: "Hello", baz: "World" } });
 * 
 * console.log(obj); // { "foo.bar": "Hello", "foo.baz": "World" }
 * ```
 */
export function flatKeys<T extends object>(
    obj: T,
    depth = 1,
    options: { flatArrayIndices?: boolean; } = {}
): OmitChildrenNodes<T> & Record<string | number | symbol, any> {
    const maxDepth = depth;
    const carrier = obj.constructor ? {} as any : Object.create(null);
    const flatArrayIndices = options?.flatArrayIndices ?? false;

    if (!isPlainObject(obj) && (!Array.isArray(obj) || !flatArrayIndices)) {
        return obj;
    }

    (function process(target: any, path: string, depth: number) {
        if (depth === maxDepth) {
            carrier[path] = target;
        } else if (Array.isArray(target) && depth) {
            if (!flatArrayIndices) {
                carrier[path] = target;
            } else {
                target.forEach((value, i) => {
                    process(
                        value,
                        path ? `${path}.${i}` : String(i),
                        path ? depth + 1 : depth
                    );
                });
            }
        } else if (isPlainObject(target) || (Array.isArray(target) && !depth)) {
            Reflect.ownKeys(target).forEach(key => {
                const value = (target as any)[key];

                if (typeof key === "symbol") {
                    if (depth === 0) { // only allow top-level symbol properties
                        carrier[key] = value;
                    }
                } else {
                    process(
                        value,
                        path ? `${path}.${key}` : key,
                        path ? depth + 1 : depth
                    );
                }
            });
        } else {
            carrier[path] = target;
        }
    })(obj, "", 0);

    return carrier;
}

/**
 * Returns a new record with all entries of the given record except the ones
 * that do not match the given predicate.
 * 
 * This function is effectively as
 * `Object.fromEntries(Object.entries(obj).filter(predicate))`.
 * 
 * @example
 * ```ts
 * import { filterEntries } from "@ayonli/jsext/object";
 * 
 * const obj = { foo: "Hello", bar: "World" };
 * const result = filterEntries(obj, ([key]) => key === "foo");
 * 
 * console.log(result); // { foo: "Hello" }
 * ```
 */
export function filterEntries<T>(
    obj: Record<string, T>,
    predicate: (entry: [string, T]) => boolean
): Record<string, T> {
    return Object.fromEntries(Object.entries(obj).filter(predicate));
}

/**
 * Applies the given transformer to all entries in the given record and returns
 * a new record containing the results.
 * 
 * This function is effectively as
 * `Object.fromEntries(Object.entries(obj).map(transformer))`.
 * 
 * @example
 * ```ts
 * import { mapEntries } from "@ayonli/jsext/object";
 * 
 * const obj = { foo: "Hello", bar: "World" };
 * const result = mapEntries(obj, ([key, value]) => [key, value.toUpperCase()]);
 * 
 * console.log(result); // { foo: "HELLO", bar: "WORLD" }
 * ```
 */
export function mapEntries<T, O>(
    obj: Record<string, T>,
    transformer: (entry: [string, T]) => [string, O]
): Record<string, O> {
    return Object.fromEntries(Object.entries(obj).map(transformer));
}

/**
 * Returns a tuple of two records with the first one containing all entries of
 * the given record that match the given predicate and the second one containing
 * all that do not.
 * 
 * @example
 * ```ts
 * import { partitionEntries } from "@ayonli/jsext/object";
 * 
 * const obj = { foo: "Hello", bar: "World" };
 * const [match, rest] = partitionEntries(obj, ([key]) => key === "foo");
 * 
 * console.log(match); // { foo: "Hello" }
 * console.log(rest); // { bar: "World" }
 * ```
 */
export function partitionEntries<T>(
    record: Record<string, T>,
    predicate: (entry: [string, T]) => boolean
): [Record<string, T>, Record<string, T>] {
    const match: Record<string, T> = {};
    const rest: Record<string, T> = {};
    const entries = Object.entries(record);

    for (const [key, value] of entries) {
        if (predicate([key, value])) {
            match[key] = value;
        } else {
            rest[key] = value;
        }
    }

    return [match, rest];
}

/**
 * Composes a new record with all keys and values inverted.
 * 
 * This function is effectively as
 * `Object.fromEntries(Object.entries(record).map(([key, value]) => [value, key]))`.
 * 
 * @example
 * ```ts
 * import { invert } from "@ayonli/jsext/object";
 * 
 * const obj = { foo: "Hello", bar: "World" };
 * const result = invert(obj);
 * 
 * console.log(result); // { Hello: "foo", World: "bar" }
 * ```
 */
export function invert<T extends Record<PropertyKey, PropertyKey>>(
    record: Readonly<T>,
): { [P in keyof T as T[P]]: P; } {
    return Object.fromEntries(
        Object.entries(record).map(([key, value]) => [value, key]),
    );
}
