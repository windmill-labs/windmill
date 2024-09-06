/**
 * Calls a function safely and return errors when captured.
 * @module
 */

// @ts-ignore
import { isAsyncGenerator, isGenerator } from "./external/check-iterable/index.mjs";
import { ThenableAsyncGenerator } from "./external/thenable-generator/index.ts";

// The following declarations shall be order from complex to simple, otherwise
// TypeScript won't work well.

/**
 * Invokes an async generator function and renders its yield value and result in
 * an `[err, val]` tuple.
 * 
 * @example
 * ```ts
 * import _try from "@ayonli/jsext/try";
 * 
 * const iter = _try(async function* () {
 *     // do something that may fail
 * });
 * 
 * for await (const [err, val] of iter) {
 *     if (err) {
 *         console.error("something went wrong:", err);
 *     } else {
 *         console.log("current value:", val);
 *     }
 * }
 * ```
 */
export default function _try<E = unknown, T = any, A extends any[] = any[], TReturn = any, TNext = unknown>(
    fn: (...args: A) => AsyncGenerator<T, TReturn, TNext>,
    ...args: A
): AsyncGenerator<[E, T], [E, TReturn], TNext>;
/**
 * Invokes a generator function and renders its yield value and result in an
 * `[err, val]` tuple.
 * 
 * @example
 * ```ts
 * import _try from "@ayonli/jsext/try";
 * 
 * const iter = _try(function* () {
 *     // do something that may fail
 * });
 * 
 * for (const [err, val] of iter) {
 *     if (err) {
 *         console.error("something went wrong:", err);
 *     } else {
 *         console.log("current value:", val);
 *     }
 * }
 * ```
 */
export default function _try<E = unknown, T = any, A extends any[] = any[], TReturn = any, TNext = unknown>(
    fn: (...args: A) => Generator<T, TReturn, TNext>,
    ...args: A
): Generator<[E, T], [E, TReturn], TNext>;
/**
 * Invokes an async function and renders its result in an `[err, res]` tuple.
 * 
 * @example
 * ```ts
 * import _try from "@ayonli/jsext/try";
 * import axios from "axios";
 * 
 * let [err, res] = await _try(async () => {
 *     return await axios.get("https://example.org");
 * });
 * 
 * if (err) {
 *     res = (err as any)["response"];
 * }
 * ```
 */
export default function _try<E = unknown, R = any, A extends any[] = any[]>(
    fn: (...args: A) => Promise<R>,
    ...args: A
): Promise<[E, R]>;
/**
 * Invokes a function and renders its result in an `[err, res]` tuple.
 * 
 * @example
 * ```ts
 * import _try from "@ayonli/jsext/try";
 * 
 * const [err, res] = _try(() => {
 *     // do something that may fail
 * });
 * ```
 */
export default function _try<E = unknown, R = any, A extends any[] = any[]>(
    fn: (...args: A) => R,
    ...args: A
): [E, R];
/**
 * Resolves an async generator and renders its yield value and result in an
 * `[err, val]` tuple.
 * 
 * @example
 * ```ts
 * import _try from "@ayonli/jsext/try";
 * 
 * async function* gen() {
 *     // do something that may fail
 * }
 * 
 * for await (const [err, val] of _try(gen())) {
 *     if (err) {
 *         console.error("something went wrong:", err);
 *     } else {
 *         console.log("current value:", val);
 *     }
 * }
 * ```
 */
export default function _try<E = unknown, T = any, TReturn = any, TNext = unknown>(
    gen: AsyncGenerator<T, TReturn, TNext>
): AsyncGenerator<[E, T], [E, TReturn], TNext>;
/**
 * Resolves a generator and renders its yield value and result in an `[err, val]`
 * tuple.
 * 
 * @example
 * ```ts
 * import _try from "@ayonli/jsext/try";
 * import { range } from "@ayonli/jsext/number";
 * 
 * const iter = range(1, 10);
 * 
 * for (const [err, val] of _try(iter)) {
 *     if (err) {
 *         console.error("something went wrong:", err);
 *     } else {
 *         console.log("current value:", val);
 *     }
 * }
 * ```
 */
export default function _try<E = unknown, T = any, TReturn = any, TNext = unknown>(
    gen: Generator<T, TReturn, TNext>
): Generator<[E, T], [E, TReturn], TNext>;
/**
 * Resolves a promise and renders its result in an `[err, res]` tuple.
 * 
 * @example
 * ```ts
 * import _try from "@ayonli/jsext/try";
 * import axios from "axios";
 * 
 * let [err, res] = await _try(axios.get("https://example.org"));
 * 
 * if (err) {
 *     res = (err as any)["response"];
 * }
 * ```
 */
export default function _try<E = unknown, R = any>(job: PromiseLike<R>): Promise<[E, R]>;
export default function _try(fn: any, ...args: any[]) {
    if (typeof fn === "function") {
        try {
            return _try((fn as (...args: any[]) => any).apply(void 0, args));
        } catch (err) {
            return [err, undefined];
        }
    }

    let returns = fn;
    // Implementation details should be ordered from complex to simple.

    if (returns instanceof ThenableAsyncGenerator) {
        // special case
        returns = (returns as PromiseLike<any>)
            .then((value: any) => [null, value]);
        return Promise.resolve(returns)
            .catch((err: unknown) => [err, undefined]) as any;
    } else if (isAsyncGenerator(returns)) {
        // @ts-ignore
        return (async function* () {
            let input: unknown;
            let result: any;

            // Use `while` loop instead of `for...of...` in order to
            // retrieve the return value of a generator function.
            while (true) {
                try {
                    const { done, value } = await returns.next(input);

                    if (done) {
                        result = value;
                        break;
                    } else {
                        // Receive any potential input value that passed
                        // to the outer `next()` call, and pass them to
                        // `res.next()` in the next call.
                        input = yield Promise.resolve([null, value]);
                    }
                } catch (err) {
                    // If any error occurs, yield that error as resolved
                    // and break the loop immediately, indicating the
                    // process is forced broken.
                    yield Promise.resolve([err, undefined]);
                    break;
                }
            }

            return [null, result];
        })() as AsyncGenerator<unknown, any, unknown>;
    } else if (isGenerator(returns)) {
        return (function* () {
            let input: unknown;
            let result: any;

            while (true) {
                try {
                    const { done, value } = returns.next(input);

                    if (done) {
                        result = value;
                        break;
                    } else {
                        input = yield [null, value];
                    }
                } catch (err) {
                    yield [err, undefined];
                    break;
                }
            }

            return [null, result];
        })() as Generator<unknown, any, unknown>;
    } else if (typeof returns?.then === "function") {
        returns = (returns as PromiseLike<any>)
            .then((value: any) => [null, value]);
        return Promise.resolve(returns)
            .catch((err: unknown) => [err, undefined]) as any;
    } else {
        return [null, returns];
    }
}
