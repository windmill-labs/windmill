/**
 * Functions for parsing JSONs to specific structures.
 * @module
 */

import { Constructor } from "./types.ts";
import { isValid } from "./object.ts";
import { fromObject } from "./error.ts";
import bytes from "./bytes.ts";

const typeRegistry = new Map<Object, { [prop: string | symbol]: Constructor<any>; }>();

/**
 * Converts a JSON string into an object of the given type.
 * 
 * The type can be `String`, `Number`, `BigInt`, `Boolean`, `Date`, `Array`, `Object` or any
 * class constructor (including Error types). If the class has a static
 * `fromJSON(data: any): T` method, it will be invoked to create the instance (unless the
 * parsed `data` is `null`, which will be skipped without further processing).
 * 
 * This function generally does not perform loose conversion between types, for example,
 * `parseAs('"123"', Number)` will not work, it only reverses to the same type before the
 * data are encoded.
 * 
 * However, for compatibility support, there are some exceptions allowed, which are:
 * 
 * - `string` => `Date`
 * - `number` or `string` => `bigint`
 * - `array` => `Buffer` or `TypedArray` (e.g. `Uint8Array`), when the data only contains
 *  integers.
 * - `object` => `Buffer` or `TypedArray` (e.g. `Uint8Array`), if the data are encoded by
 *  `JSON.stringify()`.
 * - customized in `fromJSON()`
 * 
 * If the data cannot be converted to the given type, this function returns `null`.
 */
export function parseAs(text: string, type: StringConstructor): string | null;
export function parseAs(text: string, type: NumberConstructor): number | null;
export function parseAs(text: string, type: BigIntConstructor): bigint | null;
export function parseAs(text: string, type: BooleanConstructor): boolean | null;
export function parseAs<T>(text: string, type: Constructor<T> & { fromJSON?(data: any): T; }): T | null;
export function parseAs<T>(
    text: string,
    type: Function & { fromJSON?(data: any): T; }
): T | null {
    const data = JSON.parse(text);
    return as(data, type as any) as T | null;
}

/**
 * Converts the data into the given type.
 * 
 * This function is primarily used in {@link parseAs} and shares the same conversion rules, but it
 * can be used in other scenarios too, for example, inside the `fromJSON` function.
 */
export function as(data: unknown, type: StringConstructor): string | null;
export function as(data: unknown, type: NumberConstructor): number | null;
export function as(data: unknown, type: BigIntConstructor): bigint | null;
export function as(data: unknown, type: BooleanConstructor): boolean | null;
export function as<T>(data: unknown, type: Constructor<T> & { fromJSON?(data: any): T; }): T | null;
export function as(data: unknown, type: any): any {
    if (data === null || data === undefined) {
        return null;
    } else if (typeof type.fromJSON === "function") {
        return type.fromJSON(data);
    } else if (typeof data === "boolean") {
        if (type === Boolean) {
            return data;
        } else {
            return null;
        }
    } else if (typeof data === "number") {
        if (type === Number) {
            return data;
        } else if (type === BigInt) {
            try {
                return BigInt(data);
            } catch {
                return null;
            }
        } else {
            return null;
        }
    } else if (typeof data === "string") {
        if (type === String) {
            return data;
        } else if (type === Date) {
            const date = new Date(data);
            return isValid(date) ? date : null;
        } else if (type === BigInt) {
            try {
                return BigInt(data);
            } catch {
                return null;
            }
        } else {
            return null;
        }
    } else if (Array.isArray(data)) {
        if (type === Array) { // Array
            return data;
        } else if (type.prototype instanceof Array) { // Array-derivative
            return (type as ArrayConstructor).from(data);
        } else if (typeof (type.prototype as any)[Symbol.iterator] === "function" &&
            typeof (type as any)["from"] === "function"
        ) { // ArrayLike or TypedArray
            try {
                return (type as ArrayConstructor).from(data);
            } catch {
                return null;
            }
        } else {
            return null;
        }
    } else if (!([String, Number, Boolean, Date, Array] as Function[]).includes(type)) {
        if ((data as any).type === "Buffer" && Array.isArray((data as any).data)) {
            // Node.js Buffer
            if (typeof Buffer === "function" && type === Buffer) {
                try {
                    return Buffer.from((data as any).data);
                } catch {
                    return null;
                }
            } else if (typeof (type.prototype as any)[Symbol.iterator] === "function"
                && typeof (type as any)["from"] === "function"
            ) {
                try {
                    // Convert Node.js Buffer to TypedArray.
                    return (type as ArrayConstructor).from((data as any).data);
                } catch {
                    return null;
                }
            } else {
                return null;
            }
        } else if ((data as any).type === "ByteArray" && Array.isArray((data as any).data)) {
            // ByteArray
            try {
                return bytes((data as any).data);
            } catch {
                return null;
            }
        }

        const keys = Object.getOwnPropertyNames(data);
        const values = Object.values(data);

        if (keys.slice(0, 50).map(Number).every(i => !Number.isNaN(i)) &&
            values.slice(0, 50).map(Number).every(i => !Number.isNaN(i)) &&
            typeof (type.prototype as any)[Symbol.iterator] === "function" &&
            typeof (type as any)["from"] === "function"
        ) {
            // Assert the data is a TypedArray.
            try {
                return (type as ArrayConstructor).from(Object.values(data));
            } catch {
                return null;
            }
        } else if (type.prototype instanceof Error) {
            const err = fromObject(data);

            if (err) {
                // Support @JSON.type() decorator in Error constructors.
                const typeRecords = typeRegistry.get(type.prototype as Object);

                if (typeRecords) {
                    for (const key of Reflect.ownKeys(data)) {
                        const ctor = typeRecords[key];

                        if (ctor) {
                            (err as any)[key] = as((data as any)[key], ctor);
                        }
                    }
                }
            }

            return err;
        } else {
            const ins = Object.create(type.prototype as Object);
            const typeRecords = typeRegistry.get(type.prototype as Object);

            if (typeRecords) {
                for (const key of Reflect.ownKeys(data)) {
                    const ctor = typeRecords[key];
                    ins[key] = ctor ? as((data as any)[key], ctor) : (data as any)[key];
                }
            } else {
                Object.assign(ins, data);
            }

            return ins;
        }
    } else {
        return null;
    }
}

/**
 * A decorator to instruct that the target property in the class is of a specific type.
 * 
 * When parsing JSON via {@link parseAs}, this property is guaranteed to be of the given type.
 * 
 * **NOTE:** This decorator only supports TypeScript's `experimentalDecorators`.
 * 
 * @example
 * ```ts
 * import { type } from "@ayonli/jsext/json";
 * 
 * class Example {
 *     \@type(Date)
 *     date: Date;
 * }
 * ```
 */
export function type(ctor: Constructor<any>): PropertyDecorator {
    return (proto, prop) => {
        const record = typeRegistry.get(proto);

        if (record) {
            record[prop] = ctor;
        } else {
            typeRegistry.set(proto, { [prop]: ctor });
        }
    };
}
