import { unrefTimer } from './runtime.js';
import Exception from './error/Exception.js';

/**
 * Functions for async/promise context handling.
 * @module
 */
/**
 * Creates a promise that can be resolved or rejected manually.
 *
 * This function is like `Promise.withResolvers` but less verbose.
 *
 * @example
 * ```ts
 * import { asyncTask } from "@ayonli/jsext/async";
 *
 * const task = asyncTask<number>();
 *
 * setTimeout(() => task.resolve(42), 1000);
 *
 * const result = await task;
 * console.log(result); // 42
 * ```
 */
function asyncTask() {
    let resolve;
    let reject;
    const promise = new Promise((res, rej) => {
        resolve = res;
        reject = rej;
    });
    return Object.assign(promise, {
        resolve: resolve,
        reject: reject
    });
}
function abortable(task, signal) {
    if (typeof task[Symbol.asyncIterator] === "function" &&
        typeof task.then !== "function" // this skips ThenableAsyncGenerator
    ) {
        return abortableAsyncIterable(task, signal);
    }
    else {
        return select([task], signal);
    }
}
async function* abortableAsyncIterable(task, signal) {
    if (signal.aborted) {
        throw signal.reason;
    }
    const aTask = asyncTask();
    const handleAbort = () => aTask.reject(signal.reason);
    signal.addEventListener("abort", handleAbort, { once: true });
    const it = task[Symbol.asyncIterator]();
    while (true) {
        const race = Promise.race([it.next(), aTask]);
        race.catch(() => {
            signal.removeEventListener("abort", handleAbort);
        });
        const { done, value } = await race;
        if (done) {
            signal.removeEventListener("abort", handleAbort);
            return value; // async iterator may return a value
        }
        else {
            yield value;
        }
    }
}
function createTimeoutError(ms) {
    if (typeof DOMException === "function") {
        return new DOMException(`operation timeout after ${ms}ms`, "TimeoutError");
    }
    else {
        return new Exception(`operation timeout after ${ms}ms`, {
            name: "TimeoutError",
            code: 408,
        });
    }
}
/**
 * Try to resolve a promise with a timeout limit.
 *
 * @example
 * ```ts
 * import { timeout, sleep } from "@ayonli/jsext/async";
 *
 * const task = sleep(1000);
 *
 * await timeout(task, 500); // throws TimeoutError after 500ms
 * ```
 */
async function timeout(task, ms) {
    const result = await Promise.race([
        task,
        new Promise((_, reject) => unrefTimer(setTimeout(() => {
            reject(createTimeoutError(ms));
        }, ms)))
    ]);
    return result;
}
/**
 * Resolves a promise only after the given duration.
 *
 * @example
 * ```ts
 * import { after } from "@ayonli/jsext/async";
 *
 * const task = fetch("https://example.com")
 * const res = await after(task, 1000);
 *
 * console.log(res); // the response will not be printed unless 1 second has passed
 * ```
 */
async function after(task, ms) {
    const [result] = await Promise.allSettled([
        task,
        new Promise(resolve => setTimeout(resolve, ms))
    ]);
    if (result.status === "fulfilled") {
        return result.value;
    }
    else {
        throw result.reason;
    }
}
/**
 * Blocks the context for a given duration.
 *
 * @example
 * ```ts
 * import { sleep } from "@ayonli/jsext/async";
 *
 * console.log("Hello");
 *
 * await sleep(1000);
 * console.log("World"); // "World" will be printed after 1 second
 * ```
 */
async function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}
/**
 * Blocks the current routine until the test returns a truthy value, which is
 * not `false`, `null` or `undefined`. If the test throws an error, it will be
 * treated as a falsy value and the check continues.
 *
 * This functions returns the same result as the test function when passed.
 *
 * @example
 * ```ts
 * import { until } from "@ayonli/jsext/async";
 *
 * // wait for the header element to be present in the DOM
 * const ele = await until(() => document.querySelector("header"));
 * ```
 */
async function until(test) {
    return new Promise((resolve) => {
        let ongoing = false;
        const timer = setInterval(async () => {
            if (ongoing)
                return;
            try {
                ongoing = true;
                const result = await test();
                if (result !== false && result !== null && result !== undefined) {
                    clearInterval(timer);
                    resolve(result);
                }
            }
            catch (_a) {
                // ignore
            }
            finally {
                ongoing = false;
            }
        }, 1);
    });
}
/**
 * Runs multiple tasks concurrently and returns the result of the first task that
 * completes. The rest of the tasks will be aborted.
 *
 * @param tasks An array of promises or functions that return promises.
 * @param signal A parent abort signal, if provided and aborted before any task
 *  completes, the function will reject immediately with the abort reason and
 *  cancel all tasks.
 *
 * @example
 * ```ts
 * // fetch example
 * import { select } from "@ayonli/jsext/async";
 *
 * const res = await select([
 *     signal => fetch("https://example.com", { signal }),
 *     signal => fetch("https://example.org", { signal }),
 *     fetch("https://example.net"), // This task cannot actually be aborted, but ignored
 * ]);
 *
 * console.log(res); // the response from the first completed fetch
 * ```
 *
 * @example
 * ```ts
 * // with parent signal
 * import { select } from "@ayonli/jsext/async";
 *
 * const signal = AbortSignal.timeout(1000);
 *
 * try {
 *     const res = await select([
 *         signal => fetch("https://example.com", { signal }),
 *         signal => fetch("https://example.org", { signal }),
 *     ], signal);
 * } catch (err) {
 *     if ((err as Error).name === "TimeoutError") {
 *         console.error(err); // Error: signal timed out
 *     } else {
 *         throw err;
 *     }
 * }
 * ```
 */
async function select(tasks, signal = undefined) {
    if (signal === null || signal === void 0 ? void 0 : signal.aborted) {
        throw signal.reason;
    }
    const ctrl = signal ? abortWith(signal) : new AbortController();
    const { signal: _signal } = ctrl;
    tasks = [
        ...tasks,
        new Promise((_, reject) => {
            _signal.addEventListener("abort", () => {
                reject(_signal.reason);
            }, { once: true });
        })
    ];
    return await Promise.race(tasks.map(task => {
        return typeof task === "function" ? task(_signal) : task;
    })).finally(() => {
        _signal.aborted || ctrl.abort();
    });
}
function abortWith(_parent, options = undefined) {
    let parent;
    let timeout;
    if (_parent instanceof AbortSignal) {
        parent = _parent;
        timeout = options === null || options === void 0 ? void 0 : options.timeout;
    }
    else if (_parent) {
        parent = _parent.parent;
        timeout = _parent.timeout;
    }
    if (!parent && !timeout) {
        throw new TypeError("Must provide a parent signal or a timeout value, or both");
    }
    const ctrl = new AbortController();
    const { signal } = ctrl;
    if (parent) {
        if (parent.aborted) {
            ctrl.abort(parent.reason);
            return ctrl;
        }
        const abort = () => {
            signal.aborted || ctrl.abort(parent.reason);
        };
        parent.addEventListener("abort", abort, { once: true });
        signal.addEventListener("abort", () => {
            parent.aborted || parent.removeEventListener("abort", abort);
        });
    }
    if (timeout) {
        const timer = setTimeout(() => {
            signal.aborted || ctrl.abort(createTimeoutError(timeout));
        }, timeout);
        signal.addEventListener("abort", () => {
            clearTimeout(timer);
        }, { once: true });
    }
    return ctrl;
}

export { abortWith, abortable, after, asyncTask, select, sleep, timeout, until };
//# sourceMappingURL=async.js.map
