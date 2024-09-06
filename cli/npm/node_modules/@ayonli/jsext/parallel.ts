/**
 * This module provides JavaScript the ability to run functions in parallel
 * threads and take advantage of multi-core CPUs, inspired by Golang.
 * @module
 */

import { ThenableAsyncGenerator, ThenableAsyncGeneratorLike } from "./external/thenable-generator/index.ts";
import chan, { Channel } from "./chan.ts";
import { serial } from "./number.ts";
import { cwd, isFsPath, toFileUrl } from "./path.ts";
import { asyncTask } from "./async.ts";
import { isMainThread, isNode } from "./env.ts";
import { BunWorker, CallRequest, NodeWorker } from "./parallel/types.ts";
import { resolveModule, sanitizeModuleId } from "./parallel/module.ts";
import { acquireWorker, remoteTasks, wrapArgs } from "./parallel/threads.ts";

export type FunctionPropertyNames<T> = {
    [K in keyof T]: T[K] extends (...args: any[]) => any ? K : never;
}[keyof T];
export type FunctionProperties<T> = Readonly<Pick<T, FunctionPropertyNames<T>>>;
export type ThreadedFunctions<M, T extends FunctionProperties<M> = FunctionProperties<M>> = {
    [K in keyof T]: T[K] extends (...args: infer A) => AsyncGenerator<infer T, infer R, infer N> ? (...args: A) => AsyncGenerator<T, R, N>
    : T[K] extends (...args: infer A) => Generator<infer T, infer R, infer N> ? (...args: A) => AsyncGenerator<T, R, N>
    : T[K] extends (...args: infer A) => infer R ? (...args: A) => Promise<Awaited<R>>
    : T[K];
};

const taskIdCounter = serial(true);

async function safeRemoteCall(
    worker: Worker | BunWorker | NodeWorker,
    req: CallRequest,
    transferable: ArrayBuffer[],
    taskId: number
) {
    try {
        (worker as Worker).postMessage(req, transferable);
    } catch (err) {
        remoteTasks.delete(taskId);

        if (typeof (worker as any)["unref"] === "function") {
            (worker as BunWorker | NodeWorker).unref();
        }

        throw err;
    }
}

function createRemoteCall(module: string, fn: string, args: any[]) {
    const taskId = taskIdCounter.next().value as number;
    remoteTasks.set(taskId, { module, fn });

    let getWorker = acquireWorker(taskId, parallel);
    const { args: _args, transferable } = wrapArgs(args, getWorker);

    getWorker = getWorker.then(async (worker) => {
        await safeRemoteCall(worker, {
            type: "call",
            module,
            fn,
            args: _args,
            taskId,
        }, transferable, taskId);

        return worker;
    });

    return new ThenableAsyncGenerator({
        then(onfulfilled, onrejected) {
            const task = remoteTasks.get(taskId)!;

            if (task.error) {
                remoteTasks.delete(taskId);
                return onrejected?.(task.error);
            } else if (task.result) {
                remoteTasks.delete(taskId);
                return onfulfilled?.(task.result.value);
            } else {
                return getWorker.then(() => {
                    return (task.promise = asyncTask()).finally(() => {
                        remoteTasks.delete(taskId);
                    });
                }).then(onfulfilled, onrejected);
            }
        },
        async next(input) {
            const task = remoteTasks.get(taskId)!;

            if (task.error) {
                const err = task.error;
                remoteTasks.delete(taskId);
                throw err;
            } else if (task.result) {
                const value = task.result.value;
                remoteTasks.delete(taskId);
                return { value, done: true };
            } else {
                task.channel ??= chan(Infinity);
                const worker = await getWorker;

                if (!task.generate) {
                    await new Promise<void>(resolve => {
                        task.generate = resolve;
                    });
                }

                const { args, transferable } = wrapArgs([input], getWorker);

                await safeRemoteCall(worker, {
                    type: "next",
                    args: args,
                    taskId,
                }, transferable, taskId);

                return await task.channel.recv();
            }
        },
        async return(value) {
            remoteTasks.delete(taskId);
            const worker = await getWorker;
            const { args, transferable } = wrapArgs([value], getWorker);

            await safeRemoteCall(worker, {
                type: "return",
                args: args,
                taskId,
            }, transferable, taskId);

            return { value, done: true };
        },
        async throw(err) {
            remoteTasks.delete(taskId);
            const worker = await getWorker;

            await safeRemoteCall(worker, {
                type: "throw",
                args: [err],
                taskId,
            }, transferable, taskId);

            throw err;
        },
    } as ThenableAsyncGeneratorLike);
}

function createLocalCall(module: string, fn: string, args: any[]) {
    const getReturns = resolveModule(module).then(mod => {
        return mod[fn](...args);
    });

    return new ThenableAsyncGenerator({
        then(onfulfilled, onrejected) {
            return getReturns.then(onfulfilled, onrejected);
        },
        async next(input) {
            const gen: Generator | AsyncGenerator = await getReturns;
            return await gen.next(input);
        },
        async return(value) {
            const gen: Generator | AsyncGenerator = await getReturns;
            return await gen.return(value);
        },
        async throw(err) {
            const gen: Generator | AsyncGenerator = await getReturns;
            return gen.throw(err);
        },
    } as ThenableAsyncGeneratorLike);
}

function extractBaseUrl(stackTrace: string): string | undefined {
    let lines = stackTrace.split("\n");
    const offset = lines.findIndex(line => line === "Error");

    if (offset !== -1 && offset !== 0) {
        lines = lines.slice(offset); // fix for tsx in Node.js v16
    }

    let callSite: string;

    if (lines[0]?.startsWith("Error")) { // chromium browsers
        callSite = lines[2] as string;
    } else {
        callSite = lines[1] as string;
    }

    let baseUrl: string | undefined;

    if (callSite) {
        let start = callSite.lastIndexOf("(");
        let end = 0;

        if (start !== -1) {
            start += 1;
            end = callSite.indexOf(")", start);
            callSite = callSite.slice(start, end);
        } else if (callSite.startsWith("    at ")) {
            callSite = callSite.slice(7); // remove leading `    at `
        } else if (typeof location === "object") { // general browsers
            start = callSite.match(/(https?|file):/)?.index ?? -1;

            if (start > 0) {
                callSite = callSite.slice(start);
            }
        }

        baseUrl = callSite.replace(/:\d+:\d+$/, "");

        if (!/^(https?|file):/.test(baseUrl)) {
            if (isFsPath(baseUrl)) {
                baseUrl = toFileUrl(baseUrl);
            } else {
                try {
                    baseUrl = toFileUrl(cwd()) + "/"; // must ends with `/`
                } catch { // `cwd()` may fail in unsupported environments or being rejected
                    baseUrl = "";
                }
            }
        }
    }

    return baseUrl;
}

/**
 * Wraps a module so its functions will be run in worker threads.
 * 
 * In Node.js and Bun, the `module` can be either an ES module or a CommonJS
 * module, **node_modules** and built-in modules are also supported.
 * 
 * In browsers and Deno, the `module` can only be an ES module.
 * 
 * Data are cloned and transferred between threads via **Structured Clone**
 * **Algorithm**.
 * 
 * Apart from the standard data types supported by the algorithm, {@link Channel}
 * can also be used to transfer data between threads. To do so, just passed a
 * channel instance to the threaded function. But be aware, channel can only be
 * used as a parameter, return a channel from the threaded function is not
 * allowed. Once passed, the data can only be transferred into and out-from the
 * function.
 * 
 * The difference between using a channel and a generator function for streaming
 * processing is, for a generator function, `next(value)` is coupled with a
 * `yield value`, the process is blocked between **next** calls, channel doesn't
 * have this limit, we can use it to stream all the data into the function
 * before processing and receiving any result.
 * 
 * The threaded function also supports `ArrayBuffer`s as transferable objects.
 * If an array buffer is presented as an argument or the direct property of an
 * argument (assume it's a plain object), or the array buffer is the return
 * value or the direct property of the return value (assume it's a plain object),
 * it automatically becomes a transferrable object and will be transferred to
 * the other thread instead of being cloned. This strategy allows us to easily
 * compose objects like `Request` and `Response` instances into plain objects
 * and pass them between threads without overhead.
 * 
 * **NOTE:**
 * If the current module is already in a worker thread, use this function won't
 * create another worker thread.
 * 
 * **NOTE:**
 * Cloning and transferring data between the main thread and worker threads are
 * very heavy and slow, worker threads are only intended to run CPU-intensive
 * tasks or divide tasks among multiple threads, they have no advantage when
 * performing IO-intensive tasks such as handling HTTP requests, always prefer
 * `cluster` module for that kind of purpose.
 * 
 * **NOTE:**
 * For error instances, only the following types are guaranteed to be sent and
 * received properly between threads.
 * 
 * - `Error`
 * - `EvalError`
 * - `RangeError`
 * - `ReferenceError`
 * - `SyntaxError`
 * - `TypeError`
 * - `URIError`
 * - `AggregateError` (as arguments, return values, thrown values, or shallow
 *   object properties)
 * - `Exception` (as arguments, return values, thrown values, or shallow object
 *   properties)
 * - `DOMException` (as arguments, return values, thrown values, or shallow
 *   object properties)
 * 
 * In order to handle errors properly between threads, throw well-known error
 * types or use `Exception` (or `DOMException`) with error names in the threaded
 * function.
 * 
 * @example
 * ```ts
 * // regular or async function
 * import parallel from "@ayonli/jsext/parallel";
 * const { greet } = parallel(() => import("./examples/worker.mjs"));
 * 
 * console.log(await greet("World")); // Hi, World
 * ```
 * 
 * @example
 * ```ts
 * // generator or async generator function
 * import parallel from "@ayonli/jsext/parallel";
 * const { sequence } = parallel(() => import("./examples/worker.mjs"));
 * 
 * for await (const word of sequence(["foo", "bar"])) {
 *     console.log(word);
 * }
 * // output:
 * // foo
 * // bar
 * ```
 * 
 * @example
 * ```ts
 * // use channel
 * import chan from "@ayonli/jsext/chan";
 * import { range } from "@ayonli/jsext/number";
 * import readAll from "@ayonli/jsext/readAll";
 * import parallel from "@ayonli/jsext/parallel";
 * const { twoTimesValues } = parallel(() => import("./examples/worker.mjs"));
 * 
 * const channel = chan<{ value: number; done: boolean; }>();
 * const length = twoTimesValues(channel);
 * 
 * for (const value of range(0, 9)) {
 *     await channel.push({ value, done: value === 9 });
 * }
 * 
 * const results = (await readAll(channel)).map(item => item.value);
 * console.log(results);      // [0, 2, 4, 6, 8, 10, 12, 14, 16, 18]
 * console.log(await length); // 10
 * ```
 * 
 * @example
 * ```ts
 * // use transferrable
 * import parallel from "@ayonli/jsext/parallel";
 * const { transfer } = parallel(() => import("./examples/worker.mjs"));
 * 
 * const arr = Uint8Array.from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
 * const length = await transfer(arr.buffer);
 * 
 * console.log(length);     // 10
 * console.log(arr.length); // 0
 * ```
 * 
 * **Compatibility List:**
 * 
 * The following environments are guaranteed to work:
 * 
 * - [x] Node.js v12+
 * - [x] Deno v1.0+
 * - [x] Bun v1.0+
 * - [x] Modern browsers
 * 
 * **Use with Vite:**
 * 
 * In order to use parallel threads with Vite, we need to adjust a little bit,
 * please check [this document](https://github.com/ayonli/jsext/blob/main/parallel/README.md#use-with-vite).
 * 
 * **Warn about TSX (the runtime):**
 * 
 * For users who use `tsx` to run TypeScript directly in Node.js, the runtime is
 * unable to use TypeScript directly in worker threads at the moment, so this
 * function won't work in such a case.
 * See [this issue](https://github.com/privatenumber/tsx/issues/354) for more
 * information. 
 */
function parallel<M extends { [x: string]: any; }>(
    module: string | (() => Promise<M>)
): ThreadedFunctions<M> {
    if (!isNode && typeof Worker !== "function") {
        throw new Error("Unsupported runtime");
    }

    let modId = sanitizeModuleId(module, true);
    let baseUrl: string | undefined;

    if (isFsPath(modId)) {
        if (typeof Error.captureStackTrace === "function") {
            const trace: { stack?: string; } = {};
            Error.captureStackTrace(trace);
            baseUrl = extractBaseUrl(trace.stack as string);
        } else {
            const trace = new Error("");
            baseUrl = extractBaseUrl(trace.stack as string);
        }
    }

    if (baseUrl) {
        modId = new URL(modId, baseUrl).href;
    }

    return new Proxy(Object.create(null), {
        get: (target, prop: string | symbol) => {
            if (Reflect.has(target, prop)) {
                return target[prop];
            } else if (typeof prop === "symbol") {
                return undefined;
            }

            const obj = {
                // This syntax will give our remote function a name.
                [prop]: (...args: Parameters<M["_"]>) => {
                    if (isMainThread) {
                        return createRemoteCall(modId, prop, args);
                    } else {
                        return createLocalCall(modId, prop, args);
                    }
                }
            };

            return obj[prop];
        }
    }) as any;
}

namespace parallel {
    /**
     * The maximum number of workers allowed to exist at the same time. If not
     * set, the program by default uses CPU core numbers as the limit.
     */
    export var maxWorkers: number | undefined = undefined;

    /**
     * In browsers, by default, the program loads the worker entry directly from
     * GitHub, which could be slow due to poor internet connection, we can copy
     * the entry file `bundle/worker.mjs` to a local path of our website and set
     * this option to that path so that it can be loaded locally.
     * 
     * Or, if the code is bundled, the program won't be able to automatically
     * locate the entry file in the file system, in such case, we can also copy
     * the entry file (`bundle/worker.mjs` for Bun, Deno and the browser,
     * `bundle/worker-node.mjs` for Node.js) to a local directory and supply
     * this option instead.
     */
    export var workerEntry: string | undefined = undefined;

    /**
     * Indicates whether the current thread is the main thread.
     */
    export const isMainThread: boolean = false;
}

Object.defineProperty(parallel, "isMainThread", {
    value: isMainThread,
    writable: false,
});

export default parallel;
