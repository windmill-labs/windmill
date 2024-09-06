/**
 * Functions for async/promise context handling.
 * @module
 */

import { unrefTimer } from "./runtime.ts";
import Exception from "./error/Exception.ts";

/** A promise that can be resolved or rejected manually. */
export type AsyncTask<T> = Promise<T> & {
    resolve: (value: T | PromiseLike<T>) => void;
    reject: (reason?: any) => void;
};

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
export function asyncTask<T>(): AsyncTask<T> {
    let resolve: (value: T | PromiseLike<T>) => void;
    let reject: (reason?: any) => void;
    const promise = new Promise<T>((res, rej) => {
        resolve = res;
        reject = rej;
    }) as AsyncTask<T>;

    return Object.assign(promise, {
        resolve: resolve!,
        reject: reject!
    });
}

/**
 * Wraps an async iterable object with an abort signal.
 * 
 * @example
 * ```ts
 * import { abortable, sleep } from "@ayonli/jsext/async";
 * 
 * async function* generate() {
 *     yield "Hello";
 *     await sleep(1000);
 *     yield "World";
 * }
 * 
 * const iterator = generate();
 * const controller = new AbortController();
 * 
 * setTimeout(() => controller.abort(), 100);
 * 
 * // prints "Hello" and throws AbortError after 100ms
 * for await (const value of abortable(iterator, controller.signal)) {
 *     console.log(value); // "Hello"
 * }
 * ```
 */
export function abortable<T>(task: AsyncIterable<T>, signal: AbortSignal): AsyncIterable<T>;
/**
 * Try to resolve a promise with an abort signal.
 * 
 * **NOTE:** This function does not cancel the task itself, it only prematurely
 * breaks the current routine when the signal is aborted. In order to support
 * cancellation, the task must be designed to handle the abort signal itself.
 * 
 * @deprecated This signature is confusing and doesn't  actually cancel the task,
 * use {@link select} instead.
 * 
 * @example
 * ```ts
 * import { abortable, sleep } from "@ayonli/jsext/async";
 * 
 * const task = sleep(1000);
 * const controller = new AbortController();
 * 
 * setTimeout(() => controller.abort(), 100);
 * 
 * await abortable(task, controller.signal); // throws AbortError after 100ms
 * ```
 */
export function abortable<T>(task: PromiseLike<T>, signal: AbortSignal): Promise<T>;
export function abortable<T>(
    task: PromiseLike<T> | AsyncIterable<T>,
    signal: AbortSignal
): Promise<T> | AsyncIterable<T> {
    if (typeof (task as any)[Symbol.asyncIterator] === "function" &&
        typeof (task as any).then !== "function" // this skips ThenableAsyncGenerator
    ) {
        return abortableAsyncIterable(task as AsyncIterable<T>, signal);
    } else {
        return select([task as PromiseLike<T>], signal);
    }
}

async function* abortableAsyncIterable<T>(
    task: AsyncIterable<T>,
    signal: AbortSignal
): AsyncIterable<T> {
    if (signal.aborted) {
        throw signal.reason;
    }

    const aTask = asyncTask<never>();
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
        } else {
            yield value;
        }
    }
}

function createTimeoutError(ms: number): DOMException | Exception {
    if (typeof DOMException === "function") {
        return new DOMException(`operation timeout after ${ms}ms`, "TimeoutError");
    } else {
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
export async function timeout<T>(task: PromiseLike<T>, ms: number): Promise<T> {
    const result = await Promise.race([
        task,
        new Promise<T>((_, reject) => unrefTimer(setTimeout(() => {
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
export async function after<T>(task: PromiseLike<T>, ms: number): Promise<T> {
    const [result] = await Promise.allSettled([
        task,
        new Promise<void>(resolve => setTimeout(resolve, ms))
    ]);

    if (result.status === "fulfilled") {
        return result.value;
    } else {
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
export async function sleep(ms: number): Promise<void> {
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
export async function until<T>(
    test: () => T | PromiseLike<T>
): Promise<T extends false | null | undefined ? never : T> {
    return new Promise((resolve) => {
        let ongoing = false;
        const timer = setInterval(async () => {
            if (ongoing) return;

            try {
                ongoing = true;
                const result = await test();

                if (result !== false && result !== null && result !== undefined) {
                    clearInterval(timer);
                    resolve(result as any);
                }
            } catch {
                // ignore
            } finally {
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
export async function select<T>(
    tasks: (PromiseLike<T> | ((signal: AbortSignal) => PromiseLike<T>))[],
    signal: AbortSignal | undefined = undefined
): Promise<T> {
    if (signal?.aborted) {
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

/**
 * Options for {@link abortWith}.
 * 
 * NOTE: Must provide a `parent` signal or a `timeout` value, or both.
 */
export interface AbortWithOptions {
    /**
     * The parent signal to be linked with the new abort signal. If the parent
     * signal is aborted, the new signal will be aborted with the same reason.
     */
    parent?: AbortSignal;
    /**
     * If provided, the abort signal will be automatically aborted after the
     * given duration (in milliseconds) if it is not already aborted.
     */
    timeout?: number;
}

/**
 * Creates a new abort controller with a `parent` signal, the new abort signal
 * will be aborted if the controller's `abort` method is called or when the
 * parent signal is aborted, whichever happens first.
 * 
 * @example
 * ```ts
 * import { abortWith } from "@ayonli/jsext/async";
 * 
 * const parent = new AbortController();
 * const child1 = abortWith(parent.signal);
 * const child2 = abortWith(parent.signal);
 * 
 * child1.abort();
 * 
 * console.assert(child1.signal.aborted);
 * console.assert(!parent.signal.aborted);
 * 
 * parent.abort();
 * 
 * console.assert(child2.signal.aborted);
 * console.assert(child2.signal.reason === parent.signal.reason);
 * ```
 */
export function abortWith(
    parent: AbortSignal,
    options?: Omit<AbortWithOptions, "parent">
): AbortController;
export function abortWith(
    options: AbortWithOptions
): AbortController;
export function abortWith(
    _parent: AbortSignal | AbortWithOptions,
    options: Omit<AbortWithOptions, "parent"> | undefined = undefined,
): AbortController {
    let parent: AbortSignal | undefined;
    let timeout: number | undefined;

    if (_parent instanceof AbortSignal) {
        parent = _parent;
        timeout = options?.timeout;
    } else if (_parent) {
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
