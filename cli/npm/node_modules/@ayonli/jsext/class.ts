/**
 * This module includes functions for dealing with classes.
 * @module
 */

import { Constructor } from "./types.ts";
import _mixin from "./mixin.ts";

/** @deprecated import `mixin` from `@ayonli/jsext/mixin` instead. */
export const mixin = _mixin;

/**
 * Checks if a value is a class/constructor.
 * 
 * @example
 * ```ts
 * import { isClass } from "@ayonli/jsext/class";
 * 
 * console.assert(isClass(class Foo { }));
 * console.assert(!isClass(function foo() { }));
 * ```
 */
export function isClass(value: unknown): value is Constructor<any> {
    if (typeof value !== "function")
        return false;

    if (([String, Number, Boolean, BigInt, Symbol] as Function[]).includes(value)) {
        return false;
    }

    // async function or arrow function
    if (value.prototype === undefined)
        return false;

    // generator function or malformed inheritance
    if (value.prototype.constructor !== value)
        return false;

    const str = value.toString();

    // ES6 class
    if (str.slice(0, 5) === "class")
        return true;

    const name0 = value.name[0];

    if (name0 && name0 >= "A" && name0 <= "Z" && str.includes("[native code]"))
        return true;

    return false;
}

/**
 * Checks if a class is a subclass of another class.
 * 
 * @example
 * ```ts
 * import { isSubclassOf } from "@ayonli/jsext/class";
 * 
 * class Moment extends Date {}
 * 
 * console.assert(isSubclassOf(Moment, Date));
 * console.assert(isSubclassOf(Moment, Object)); // all classes are subclasses of Object
 * ```
 */
export function isSubclassOf<A, B>(ctor1: Constructor<A>, ctor2: Constructor<B>): boolean {
    return typeof ctor1 === "function"
        && typeof ctor2 === "function"
        && ctor1.prototype instanceof ctor2;
}
