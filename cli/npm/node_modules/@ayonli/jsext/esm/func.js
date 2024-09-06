import { isAsyncGenerator, isGenerator } from './external/check-iterable/index.js';

/**
 * Inspired by Golang, creates a function that receives a `defer` keyword which
 * can be used to carry deferred jobs.
 * @module
 */
// @ts-ignore
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
function func(fn) {
    return function (...args) {
        var _a;
        const callbacks = [];
        const defer = (cb) => void callbacks.push(cb);
        let result;
        try {
            const returns = fn.call(this, defer, ...args);
            if (isAsyncGenerator(returns)) {
                const gen = (async function* () {
                    var _a;
                    let input;
                    // Use `while` loop instead of `for...of...` in order to
                    // retrieve the return value of a generator function.
                    while (true) {
                        try {
                            const { done, value } = await returns.next(input);
                            if (done) {
                                result = { value, error: null };
                                break;
                            }
                            else {
                                // Receive any potential input value that passed
                                // to the outer `next()` call, and pass them to
                                // `res.next()` in the next call.
                                input = yield Promise.resolve(value);
                            }
                        }
                        catch (error) {
                            // If any error occurs, capture that error and break
                            // the loop immediately, indicating the process is
                            // forced broken.
                            result = { value: void 0, error };
                            break;
                        }
                    }
                    for (let i = callbacks.length - 1; i >= 0; i--) {
                        await ((_a = callbacks[i]) === null || _a === void 0 ? void 0 : _a.call(callbacks));
                    }
                    if (result.error) {
                        throw result.error;
                    }
                    else {
                        return result.value;
                    }
                })();
                return gen;
            }
            else if (isGenerator(returns)) {
                const gen = (function* () {
                    var _a;
                    let input;
                    while (true) {
                        try {
                            const { done, value } = returns.next(input);
                            if (done) {
                                result = { value, error: null };
                                break;
                            }
                            else {
                                input = yield value;
                            }
                        }
                        catch (error) {
                            result = { value: void 0, error };
                            break;
                        }
                    }
                    for (let i = callbacks.length - 1; i >= 0; i--) {
                        (_a = callbacks[i]) === null || _a === void 0 ? void 0 : _a.call(callbacks);
                    }
                    if (result.error) {
                        throw result.error;
                    }
                    else {
                        return result.value;
                    }
                })();
                return gen;
            }
            else if (typeof (returns === null || returns === void 0 ? void 0 : returns.then) === "function") {
                return Promise.resolve(returns).then(value => ({
                    value,
                    error: null,
                })).catch((error) => ({
                    value: void 0,
                    error,
                })).then(async (result) => {
                    var _a;
                    for (let i = callbacks.length - 1; i >= 0; i--) {
                        await ((_a = callbacks[i]) === null || _a === void 0 ? void 0 : _a.call(callbacks));
                    }
                    if (result.error) {
                        throw result.error;
                    }
                    else {
                        return result.value;
                    }
                });
            }
            else {
                result = { value: returns, error: null };
            }
        }
        catch (error) {
            result = { value: void 0, error };
        }
        for (let i = callbacks.length - 1; i >= 0; i--) {
            (_a = callbacks[i]) === null || _a === void 0 ? void 0 : _a.call(callbacks);
        }
        if (result.error) {
            throw result.error;
        }
        else {
            return result.value;
        }
    };
}

export { func as default };
//# sourceMappingURL=func.js.map
