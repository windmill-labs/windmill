/**
 * Inspired by Golang, creates a function that receives a `defer` keyword which
 * can be used to carry deferred jobs.
 * @module
 */

// @ts-ignore
import { isAsyncGenerator, isGenerator } from "./external/check-iterable/index.mjs";

/**
 * Inspired by Golang, creates a function that receives a `defer` keyword which
 * can be used to carry deferred jobs that will be run after the main function
 * is complete.
 * 
 * Multiple calls of the `defer` keyword is supported, and the callbacks are
 * called in the LIFO order. Callbacks can be async functions if the main
 * function is an async function or an async generator function, and all the
 * running procedures will be awaited.
 * 
 * @example
 * ```ts
 * import func from "@ayonli/jsext/func";
 * import * as fs from "node:fs/promises";
 * 
 * export const getVersion = func(async (defer) => {
 *     const file = await fs.open("./package.json", "r");
 *     defer(() => file.close());
 * 
 *     const content = await file.readFile("utf8");
 *     const pkg = JSON.parse(content);
 * 
 *     return pkg.version as string;
 * });
 * ```
 */
export default function func<T, R = any, A extends any[] = any[]>(
    fn: (this: T, defer: (cb: () => void) => void, ...args: A) => R
): (this: T, ...args: A) => R {
    return function (this: T, ...args: A) {
        const callbacks: (() => void)[] = [];
        const defer = (cb: () => void) => void callbacks.push(cb);
        type Result = { value?: Awaited<R>; error: unknown; };
        let result: Result | undefined;

        try {
            const returns = fn.call(this, defer, ...args) as any;

            if (isAsyncGenerator(returns)) {
                const gen = (async function* () {
                    let input: unknown;

                    // Use `while` loop instead of `for...of...` in order to
                    // retrieve the return value of a generator function.
                    while (true) {
                        try {
                            const { done, value } = await returns.next(input);

                            if (done) {
                                result = { value, error: null };
                                break;
                            } else {
                                // Receive any potential input value that passed
                                // to the outer `next()` call, and pass them to
                                // `res.next()` in the next call.
                                input = yield Promise.resolve(value);
                            }
                        } catch (error) {
                            // If any error occurs, capture that error and break
                            // the loop immediately, indicating the process is
                            // forced broken.
                            result = { value: void 0, error } as Result;
                            break;
                        }
                    }

                    for (let i = callbacks.length - 1; i >= 0; i--) {
                        await (callbacks[i] as () => void | Promise<void>)?.();
                    }

                    if (result.error) {
                        throw result.error;
                    } else {
                        return result.value;
                    }
                })() as AsyncGenerator<unknown, any, unknown>;

                return gen as R;
            } else if (isGenerator(returns)) {
                const gen = (function* () {
                    let input: unknown;

                    while (true) {
                        try {
                            const { done, value } = returns.next(input);

                            if (done) {
                                result = { value, error: null };
                                break;
                            } else {
                                input = yield value;
                            }
                        } catch (error) {
                            result = { value: void 0, error } as Result;
                            break;
                        }
                    }

                    for (let i = callbacks.length - 1; i >= 0; i--) {
                        callbacks[i]?.();
                    }

                    if (result.error) {
                        throw result.error;
                    } else {
                        return result.value;
                    }
                })() as Generator<unknown, R, unknown>;

                return gen as R;
            } else if (typeof returns?.then === "function") {
                return Promise.resolve(returns as PromiseLike<R>).then(value => ({
                    value,
                    error: null,
                } as Result)).catch((error: unknown) => ({
                    value: void 0,
                    error,
                } as Result)).then(async result => {
                    for (let i = callbacks.length - 1; i >= 0; i--) {
                        await (callbacks[i] as () => void | Promise<void>)?.();
                    }

                    if (result.error) {
                        throw result.error;
                    } else {
                        return result.value;
                    }
                }) as R;
            } else {
                result = { value: returns, error: null } as Result;
            }
        } catch (error) {
            result = { value: void 0, error } as Result;
        }

        for (let i = callbacks.length - 1; i >= 0; i--) {
            callbacks[i]?.();
        }

        if (result.error) {
            throw result.error;
        } else {
            return result.value as R;
        }
    };
}
