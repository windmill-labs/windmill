/**
 * Performs pipe operations through a series of functions upon a value.
 * @module
 */

const _value = Symbol.for("value");

/**
 * Pipeline holds a value and allows you chain operations upon it (and the result
 * of the previous operation) to get the final result, similar to the Unix pipe
 * operator `|` in shell scripting.
 */
export class Pipeline<T> {
    private [_value]: T;

    constructor(value: T) {
        this[_value] = value;
    }

    get [Symbol.toStringTag](): "Pipeline" {
        return "Pipeline";
    }

    get value(): T {
        return this[_value];
    }

    /**
     * Calls a function using the current value as its argument and returns a
     * new {@link Pipeline} instance that holds the result.
     */
    pipe<R, A extends any[] = any[]>(fn: (value: T, ...args: A) => R, ...args: A): Pipeline<R> {
        return new Pipeline(fn(this[_value], ...args));
    }

    valueOf(): T {
        return this[_value];
    }
}

/**
 * Constructs a {@link Pipeline} instance with the given value and performs pipe
 * operations upon it.
 * 
 * @example
 * ```ts
 * import pipe from "@ayonli/jsext/pipe";
 * 
 * const { value } = pipe("10")
 *     .pipe(parseInt)
 *     .pipe(Math.pow, 2)
 *     .pipe(v => v.toFixed(2));
 * 
 * console.log(`the value is ${value}`) // the value is 100.00
 * ```
 */
export default function pipe<T>(value: T): Pipeline<T> {
    return new Pipeline(value);
}
