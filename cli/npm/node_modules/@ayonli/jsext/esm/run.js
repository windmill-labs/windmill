import chan from './chan.js';
import { isPlainObject } from './object.js';
import { fromErrorEvent, fromObject } from './error.js';
import { toFileUrl, cwd } from './path.js';
import { isNode, isBrowserWindow, isBun } from './env.js';
import { sanitizeModuleId } from './parallel/module.js';
import { isChannelMessage, handleChannelMessage } from './parallel/channel.js';
import { getMaxParallelism, createWorker, wrapArgs, isCallResponse, unwrapReturnValue } from './parallel/threads.js';
import parallel from './parallel.js';
import { unrefTimer } from './runtime.js';
import { asyncTask } from './async.js';

/**
 * Runs a script in another thread and abort at any time.
 * @module
 */
const workerPools = new Map();
let gcTimer;
// The worker consumer queue is nothing but a callback list, once a worker is
// available, the runner pop a consumer and run the callback, which will retry
// gaining the worker and retry the task.
const workerConsumerQueue = [];
/**
 * Runs the given `script` in a worker thread and abort the task at any time.
 *
 * This function is similar to {@link parallel}(), many features and
 * restrictions applicable to `parallel()` are also applicable to `run()`,
 * except the following:
 *
 * 1. The `script` can only be a filename, and is relative to the current
 *   working directory (or the current URL) if not absolute.
 * 2. Only one task is allow to run at a time for one worker thread, set
 *   {@link run.maxWorkers} to allow more tasks to be run at the same time if
 *   needed.
 * 3. By default, the worker thread is dropped after the task settles, set
 *   `keepAlive` option in order to reuse it.
 * 4. This function is not intended to be used in the browser, because it takes
 *   a bare filename as argument, which will not be transformed to a proper URL
 *   if the program is to be bundled.
 *
 * @example
 * ```ts
 * // result
 * import run from "@ayonli/jsext/run";
 *
 * const job1 = await run("examples/worker.mjs", ["World"]);
 * console.log(await job1.result()); // Hello, World
 * ```
 *
 * @example
 * ```ts
 * // iterate
 * import run from "@ayonli/jsext/run";
 *
 * const job2 = await run<string, [string[]]>(
 *     "examples/worker.mjs",
 *     [["foo", "bar"]],
 *     { fn: "sequence" }
 * );
 * for await (const word of job2.iterate()) {
 *     console.log(word);
 * }
 * // output:
 * // foo
 * // bar
 * ```
 *
 * @example
 * ```ts
 * // abort
 * import run from "@ayonli/jsext/run";
 * import _try from "@ayonli/jsext/try";
 *
 * const job3 = await run<string, [string]>("examples/worker.mjs", ["foobar"], {
 *    fn: "takeTooLong",
 * });
 * await job3.abort();
 * const [err, res] = await _try(job3.result());
 * console.assert(err === null);
 * console.assert(res === undefined);
 * ```
 */
async function run(script, args, options) {
    var _a;
    if (!isNode && typeof Worker !== "function") {
        throw new Error("Unsupported runtime");
    }
    const maxWorkers = run.maxWorkers || parallel.maxWorkers || await getMaxParallelism;
    const fn = (options === null || options === void 0 ? void 0 : options.fn) || "default";
    let modId = sanitizeModuleId(script);
    let baseUrl = undefined;
    if (isBrowserWindow) {
        baseUrl = location.href;
    }
    else {
        try {
            baseUrl = toFileUrl(cwd()) + "/"; // must ends with `/`
        }
        catch ( // `cwd()` may fail in unsupported environments or being rejected
        _b) { // `cwd()` may fail in unsupported environments or being rejected
            baseUrl = "";
        }
    }
    if (baseUrl) {
        modId = new URL(modId, baseUrl).href;
    }
    const req = {
        type: "call",
        module: modId,
        fn,
        args: args !== null && args !== void 0 ? args : [],
    };
    const adapter = (options === null || options === void 0 ? void 0 : options.adapter) || "worker_threads";
    const workerPool = (_a = workerPools.get(adapter)) !== null && _a !== void 0 ? _a : workerPools.set(adapter, []).get(adapter);
    let poolRecord = workerPool.find(item => !item.busy);
    if (poolRecord) {
        poolRecord.busy = true;
        poolRecord.lastAccess = Date.now();
    }
    else if (workerPool.length < maxWorkers) {
        // Fill the worker pool regardless the current call should keep-alive
        // or not, this will make sure that the total number of workers will not
        // exceed the `run.maxWorkers`. If the the call doesn't keep-alive the
        // worker, it will be cleaned after the call.
        workerPool.push(poolRecord = {
            getWorker: createWorker({ parallel, adapter }),
            adapter,
            busy: true,
            lastAccess: Date.now(),
        });
        if (!gcTimer) {
            gcTimer = setInterval(() => {
                workerPools.forEach((workerPool, adapter) => {
                    // GC: clean long-time unused workers
                    const now = Date.now();
                    const idealItems = [];
                    workerPools.set(adapter, workerPool.filter(item => {
                        const ideal = !item.busy
                            && (now - item.lastAccess) >= 10000;
                        if (ideal) {
                            idealItems.push(item);
                        }
                        return !ideal;
                    }));
                    idealItems.forEach(async (item) => {
                        const { worker } = await item.getWorker;
                        if (typeof worker["terminate"] === "function") {
                            await worker
                                .terminate();
                        }
                        else {
                            worker.kill();
                        }
                    });
                });
            }, 1000);
            unrefTimer(gcTimer);
        }
    }
    else {
        // Put the current call in the consumer queue if there are no workers
        // available, once an existing call finishes, the queue will pop the its
        // head consumer and retry.
        return new Promise((resolve) => {
            workerConsumerQueue.push(resolve);
        }).then(() => run(modId, args, options));
    }
    let error = null;
    let result;
    let promise;
    let channel = undefined;
    let workerId;
    let release;
    let terminate = () => Promise.resolve(void 0);
    const timeout = (options === null || options === void 0 ? void 0 : options.timeout) ? setTimeout(async () => {
        const err = new Error(`operation timeout after ${options.timeout}ms`);
        error = err;
        await terminate();
        handleClose(err, true);
    }, options.timeout) : null;
    if (timeout) {
        unrefTimer(timeout);
    }
    const handleMessage = async (msg) => {
        var _a, _b;
        if (isChannelMessage(msg)) {
            await handleChannelMessage(msg);
        }
        else if (isCallResponse(msg)) {
            timeout && clearTimeout(timeout);
            if (msg.type === "return" || msg.type === "error") {
                if (msg.type === "error") {
                    const err = isPlainObject(msg.error)
                        ? ((_a = fromObject(msg.error)) !== null && _a !== void 0 ? _a : msg.error)
                        : msg.error;
                    if (err instanceof Error &&
                        (err.message.includes("not be cloned")
                            || ((_b = err.stack) === null || _b === void 0 ? void 0 : _b.includes("not be cloned")) // Node.js v16-
                            || err.message.includes("Do not know how to serialize") // JSON error
                        )) {
                        Object.defineProperty(err, "stack", {
                            configurable: true,
                            enumerable: false,
                            writable: true,
                            value: (err.stack ? err.stack + "\n    " : "")
                                + `at ${fn} (${modId})`,
                        });
                    }
                    error = err;
                }
                else {
                    result = { value: unwrapReturnValue(msg.value) };
                }
                (options === null || options === void 0 ? void 0 : options.keepAlive) || await terminate();
                handleClose(null, !(options === null || options === void 0 ? void 0 : options.keepAlive));
            }
            else if (msg.type === "yield") {
                const value = unwrapReturnValue(msg.value);
                if (msg.done) {
                    // The final message of yield event is the return value.
                    handleMessage({
                        type: "return",
                        value,
                    });
                }
                else {
                    channel === null || channel === void 0 ? void 0 : channel.send(value);
                }
            }
        }
    };
    const handleClose = (err, terminated = false) => {
        var _a, _b, _c;
        timeout && clearTimeout(timeout);
        if (!terminated) {
            // Release before resolve.
            release === null || release === void 0 ? void 0 : release();
            if (workerConsumerQueue.length) {
                // Queued consumer now has chance to gain the worker.
                (_a = workerConsumerQueue.shift()) === null || _a === void 0 ? void 0 : _a();
            }
        }
        else if (poolRecord) {
            // Clean the pool before resolve.
            // The `workerPool` of this key in the pool map may have been
            // modified by other routines, we need to retrieve the newest value.
            const remainItems = (_b = workerPools.get(adapter)) === null || _b === void 0 ? void 0 : _b.filter(record => record !== poolRecord);
            if (remainItems === null || remainItems === void 0 ? void 0 : remainItems.length) {
                workerPools.set(adapter, remainItems);
            }
            else {
                workerPools.delete(adapter);
            }
            if (workerConsumerQueue.length) {
                // Queued consumer now has chance to create new worker.
                (_c = workerConsumerQueue.shift()) === null || _c === void 0 ? void 0 : _c();
            }
        }
        if (err) {
            error !== null && error !== void 0 ? error : (error = err);
        }
        if (error) {
            if (promise) {
                promise.reject(error);
                if (channel) {
                    channel.close();
                }
            }
            else if (channel) {
                if (error instanceof Error) {
                    channel.close(error);
                }
                else if (typeof error === "string") {
                    channel.close(new Error(error));
                }
                else {
                    // @ts-ignore
                    channel.close(new Error("unknown error", { cause: error }));
                }
            }
        }
        else {
            result !== null && result !== void 0 ? result : (result = { value: void 0 });
            if (promise) {
                promise.resolve(result.value);
            }
            if (channel) {
                channel.close();
            }
        }
    };
    const safeRemoteCall = async (worker, req, transferable = []) => {
        try {
            if (typeof worker["postMessage"] === "function") {
                worker.postMessage(req, transferable);
            }
            else {
                await new Promise((resolve, reject) => {
                    worker.send(req, err => {
                        err ? reject(err) : resolve();
                    });
                });
            }
        }
        catch (err) {
            if (typeof worker["unref"] === "function") {
                worker.unref();
            }
            error = err;
            (options === null || options === void 0 ? void 0 : options.keepAlive) || await terminate();
            handleClose(null, !(options === null || options === void 0 ? void 0 : options.keepAlive));
            throw err;
        }
    };
    if (isNode || isBun) {
        if (adapter === "child_process") {
            const record = await poolRecord.getWorker;
            const worker = record.worker;
            workerId = record.workerId;
            worker.ref(); // prevent premature exit in the main thread
            worker.on("message", handleMessage);
            worker.once("exit", (code, signal) => {
                if (!error && !result) {
                    handleClose(new Error(`worker exited (${code !== null && code !== void 0 ? code : signal})`), true);
                }
            });
            release = () => {
                // allow the main thread to exit if the event loop is empty
                worker.unref();
                // Remove the event listener so that later calls will not mess
                // up.
                worker.off("message", handleMessage);
                worker.removeAllListeners("exit");
                poolRecord && (poolRecord.busy = false);
            };
            terminate = () => Promise.resolve(void worker.kill(1));
            if (error) {
                // The worker take too long to start and timeout error already
                // thrown.
                await terminate();
                throw error;
            }
            const { args } = wrapArgs(req.args, Promise.resolve(worker));
            req.args = args;
            await safeRemoteCall(worker, req);
        }
        else if (isNode) {
            const record = await poolRecord.getWorker;
            const worker = record.worker;
            const handleErrorEvent = (err) => {
                if (!error && !result) {
                    // In Node.js, worker will exit once erred.
                    handleClose(err, true);
                }
            };
            workerId = record.workerId;
            worker.ref();
            worker.on("message", handleMessage);
            worker.once("error", handleErrorEvent);
            release = () => {
                worker.unref();
                worker.off("message", handleMessage);
                worker.off("error", handleErrorEvent);
                poolRecord && (poolRecord.busy = false);
            };
            terminate = async () => void (await worker.terminate());
            if (error) {
                await terminate();
                throw error;
            }
            const { args, transferable, } = wrapArgs(req.args, Promise.resolve(worker));
            req.args = args;
            await safeRemoteCall(worker, req, transferable);
        }
        else { // isBun
            const record = await poolRecord.getWorker;
            const worker = record.worker;
            const handleCloseEvent = ((ev) => {
                if (!error && !result) {
                    handleClose(new Error(ev.reason + " (" + ev.code + ")"), true);
                }
            });
            workerId = record.workerId;
            worker.ref();
            worker.onmessage = (ev) => handleMessage(ev.data);
            worker.onerror = () => void worker.terminate(); // terminate once erred
            worker.addEventListener("close", handleCloseEvent);
            release = () => {
                worker.unref();
                worker.onmessage = null;
                // @ts-ignore
                worker.onerror = null;
                worker.removeEventListener("close", handleCloseEvent);
                poolRecord && (poolRecord.busy = false);
            };
            terminate = () => Promise.resolve(worker.terminate());
            if (error) {
                await terminate();
                throw error;
            }
            const { args, transferable, } = wrapArgs(req.args, Promise.resolve(worker));
            req.args = args;
            await safeRemoteCall(worker, req, transferable);
        }
    }
    else {
        const record = await poolRecord.getWorker;
        const worker = record.worker;
        workerId = record.workerId;
        worker.onmessage = (ev) => handleMessage(ev.data);
        worker.onerror = (ev) => {
            var _a;
            if (!error && !result) {
                worker.terminate(); // ensure termination
                handleClose((_a = fromErrorEvent(ev)) !== null && _a !== void 0 ? _a : new Error("worker exited"), true);
            }
        };
        release = () => {
            worker.onmessage = null;
            // @ts-ignore
            worker.onerror = null;
            poolRecord && (poolRecord.busy = false);
        };
        terminate = () => Promise.resolve(worker.terminate());
        if (error) {
            await terminate();
            throw error;
        }
        const { args, transferable, } = wrapArgs(req.args, Promise.resolve(worker));
        req.args = args;
        await safeRemoteCall(worker, req, transferable);
    }
    return {
        workerId,
        async abort(reason = undefined) {
            timeout && clearTimeout(timeout);
            if (reason) {
                error = reason;
            }
            else {
                result = { value: void 0 };
            }
            await terminate();
            handleClose(null, true);
        },
        async result() {
            const task = asyncTask();
            if (error) {
                task.reject(error);
            }
            else if (result) {
                task.resolve(result.value);
            }
            else {
                promise = task;
            }
            return await task;
        },
        iterate() {
            if (promise) {
                throw new Error("result() has been called");
            }
            else if (result) {
                throw new TypeError("the response is not iterable");
            }
            channel = chan(Infinity);
            return {
                [Symbol.asyncIterator]: channel[Symbol.asyncIterator].bind(channel),
            };
        },
    };
}
(function (run) {
    /**
     * The maximum number of workers allowed to exist at the same time.
     * If not set, use the same setting as {@link parallel.maxWorkers}.
     */
    run.maxWorkers = undefined;
    /** @deprecated set {@link parallel.workerEntry} instead */
    run.workerEntry = undefined;
})(run || (run = {}));
// backward compatibility
Object.defineProperties(run, {
    workerEntry: {
        set(v) {
            parallel.workerEntry = v;
        },
        get() {
            return parallel.workerEntry;
        },
    },
});
var run$1 = run;

export { run$1 as default };
//# sourceMappingURL=run.js.map
