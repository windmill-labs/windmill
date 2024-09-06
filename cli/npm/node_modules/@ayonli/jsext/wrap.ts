/**
 * Wraps a function for decorator pattern but keep its signature.
 * @module
 */

/**
 * Wraps a function inside another function and returns a new function that
 * copies the original function's name and other properties.
 * 
 * @example
 * ```ts
 * import wrap from "@ayonli/jsext/wrap";
 * 
 * function log(text: string) {
 *     console.log(text);
 * }
 * 
 * const show = wrap(log, function (fn, text) {
 *     return fn.call(this, new Date().toISOString() + " " + text);
 * });
 *  
 * console.log(show.name); // log
 * console.log(show.length); // 1
 * console.assert(show.toString() === log.toString());
 * ```
 */
export default function wrap<T, Fn extends (this: T, ...args: any[]) => any>(
    fn: Fn,
    wrapper: (this: T, fn: Fn, ...args: Parameters<Fn>) => ReturnType<Fn>
): Fn {
    const wrapped = function (this: any, ...args: Parameters<Fn>): ReturnType<Fn> {
        return wrapper.call(this, fn, ...args);
    };

    Object.defineProperty(wrapped,
        "name",
        Object.getOwnPropertyDescriptor(fn, "name") as PropertyDescriptor);
    Object.defineProperty(wrapped,
        "length",
        Object.getOwnPropertyDescriptor(fn, "length") as PropertyDescriptor);
    Object.defineProperty(wrapped, "toString", {
        configurable: true,
        enumerable: false,
        writable: true,
        value: fn.toString.bind(fn),
    });

    return wrapped as Fn;
}
