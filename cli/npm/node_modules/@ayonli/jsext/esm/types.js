/**
 * The missing utility types for TypeScript.
 * @module
 */
/**
 * This is the very constructor/class of all async functions.
 *
 * @example
 * ```ts
 * import { AsyncFunction } from "@ayonli/jsext/types";
 *
 * async function foo() {
 *       // ...
 * }
 *
 * console.assert(foo instanceof AsyncFunction);
 * ```
 */
const AsyncFunction = (async function () { }).constructor;
/**
 * This is the very constructor/class of all generator functions.
 *
 * @example
 * ```ts
 * import { GeneratorFunction } from "@ayonli/jsext/types";
 *
 * function* foo() {
 *     // ...
 * }
 *
 * console.assert(foo instanceof GeneratorFunction);
 * ```
 */
const GeneratorFunction = (function* () { }).constructor;
/**
 * This is the very constructor/class of all async generator functions.
 *
 * @example
 * ```ts
 * import { AsyncGeneratorFunction } from "@ayonli/jsext/types";
 *
 * async function* foo() {
 *    // ...
 * }
 *
 * console.assert(foo instanceof AsyncGeneratorFunction);
 * ```
 */
const AsyncGeneratorFunction = (async function* () { }).constructor;
/**
 * This is the superclass of all of all `TypedArray` subclasses, such as `Uint8Array`,
 * `Int16Array`, etc.
 *
 * @see https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/TypedArray
 *
 * @example
 * ```ts
 * import { TypedArray } from "@ayonli/jsext/types";
 *
 * const arr = new Uint8Array(10);
 * console.assert(arr instanceof TypedArray);
 * ```
 */
const TypedArray = Object.getPrototypeOf(Uint8Array);

export { AsyncFunction, AsyncGeneratorFunction, GeneratorFunction, TypedArray };
//# sourceMappingURL=types.js.map
