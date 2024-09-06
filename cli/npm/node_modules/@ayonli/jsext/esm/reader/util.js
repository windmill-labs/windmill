import chan from '../chan.js';
import { fromErrorEvent } from '../error.js';

function isFunction(val) {
    return typeof val === "function";
}
async function* resolveAsyncIterable(promise) {
    const stream = await promise;
    const reader = stream.getReader();
    try {
        while (true) {
            const { done, value } = await reader.read();
            if (done) {
                break;
            }
            yield value;
        }
    }
    finally {
        reader.releaseLock();
    }
}
function resolveReadableStream(promise) {
    const { readable, writable } = new TransformStream();
    promise.then(stream => stream.pipeTo(writable));
    return readable;
}
/**
 * If the given `promise` resolves to a `ReadableStream<Uint8Array>`, this
 * function will return a new `ReadableStream<Uint8Array>` object that can be
 * used to read the byte stream without the need to wait for the promise to
 * resolve.
 *
 * This function is optimized for zero-copy read, so it's recommended to use
 * this function when the source stream is a byte stream.
 */
function resolveByteStream(promise) {
    let reader;
    return new ReadableStream({
        type: "bytes",
        async start() {
            const source = await promise;
            try { // zero-copy read from the source stream
                reader = source.getReader({ mode: "byob" });
            }
            catch (_a) {
                reader = source.getReader();
            }
        },
        async pull(controller) {
            var _a;
            try {
                let request;
                let view;
                let result;
                if ("byobRequest" in controller && ((_a = controller.byobRequest) === null || _a === void 0 ? void 0 : _a.view)) {
                    // This stream is requested for zero-copy read.
                    request = controller.byobRequest;
                    view = request.view;
                }
                else if (reader instanceof ReadableStreamBYOBReader) {
                    view = new Uint8Array(4096);
                }
                if (reader instanceof ReadableStreamBYOBReader) {
                    // The source stream supports zero-copy read, we can read its
                    // data directly into the request view's buffer.
                    result = await reader.read(view);
                }
                else {
                    // The source stream does not support zero-copy read, we need to
                    // copy its data to a new buffer.
                    result = await reader.read();
                }
                if (request) {
                    if (result.done) {
                        controller.close();
                        // The final chunk may be empty, but still needs to be
                        // responded in order to close the request reader.
                        if (result.value !== undefined) {
                            request.respondWithNewView(result.value);
                        }
                        else {
                            request.respond(0);
                        }
                    }
                    else if (reader instanceof ReadableStreamBYOBReader
                        || (view && result.value.buffer.byteLength === view.buffer.byteLength)) {
                        // Respond to the request reader with the same underlying
                        // buffer of the source stream.
                        // Or the source stream doesn't support zero-copy read, but
                        // the result bytes has the same buffer size as the request
                        // view.
                        request.respondWithNewView(result.value);
                    }
                    else {
                        // This stream is requested for zero-copy read, but the
                        // source stream doesn't support it. We need to copy and
                        // deliver the new buffer instead.
                        controller.enqueue(result.value);
                    }
                }
                else {
                    if (result.done) {
                        controller.close();
                    }
                    else {
                        controller.enqueue(result.value);
                    }
                }
            }
            catch (err) {
                reader.releaseLock();
                controller.error(err);
            }
        },
        cancel(reason = undefined) {
            reader.cancel(reason);
            reader.releaseLock();
        },
    });
}
/**
 * Converts the given `source` into an `AsyncIterable` object if it's not one
 * already, returns `null` if failed.
 */
function asAsyncIterable(source) {
    if (typeof source[Symbol.asyncIterator] === "function") {
        return source;
    }
    else if (typeof source[Symbol.iterator] === "function") {
        return {
            [Symbol.asyncIterator]: async function* () {
                for (const value of source) {
                    yield value;
                }
            },
        };
    }
    else if (typeof ReadableStream === "function"
        && source instanceof ReadableStream) {
        const reader = source.getReader();
        return {
            [Symbol.asyncIterator]: async function* () {
                try {
                    while (true) {
                        const { done, value } = await reader.read();
                        if (done) {
                            break;
                        }
                        yield value;
                    }
                }
                catch (err) {
                    reader.cancel(err);
                }
                finally {
                    reader.releaseLock();
                }
            },
        };
    }
    else if (typeof source["then"] === "function") {
        return resolveAsyncIterable(source);
    }
    return null;
}
function toAsyncIterable(source, eventMap = undefined) {
    var _a;
    const iterable = asAsyncIterable(source);
    if (iterable) {
        return iterable;
    }
    const channel = chan(Infinity);
    const handleMessage = channel.send.bind(channel);
    const handleClose = channel.close.bind(channel);
    const handleBrowserErrorEvent = (ev) => {
        let err;
        if (typeof ErrorEvent === "function" && ev instanceof ErrorEvent) {
            err = ev.error || fromErrorEvent(ev);
        }
        else {
            // @ts-ignore
            err = new Error("something went wrong", { cause: ev });
        }
        handleClose(err);
    };
    const proto = Object.getPrototypeOf(source);
    const msgDesc = Object.getOwnPropertyDescriptor(proto, "onmessage");
    if ((msgDesc === null || msgDesc === void 0 ? void 0 : msgDesc.set) && isFunction(source["close"])) { // WebSocket or EventSource
        const errDesc = Object.getOwnPropertyDescriptor(proto, "onerror");
        const closeDesc = Object.getOwnPropertyDescriptor(proto, "onclose");
        let cleanup;
        if ((eventMap === null || eventMap === void 0 ? void 0 : eventMap.event) &&
            (eventMap === null || eventMap === void 0 ? void 0 : eventMap.event) !== "message" &&
            isFunction(source["addEventListener"])) { // for EventSource listening on custom events
            const es = source;
            const eventName = eventMap.event;
            const msgListener = (ev) => {
                handleMessage(ev.data);
            };
            es.addEventListener(eventName, msgListener);
            cleanup = () => {
                es.removeEventListener(eventName, msgListener);
            };
        }
        else {
            msgDesc.set.call(source, (ev) => {
                handleMessage(ev.data);
            });
            cleanup = () => {
                var _a;
                (_a = msgDesc.set) === null || _a === void 0 ? void 0 : _a.call(source, null);
            };
        }
        (_a = errDesc === null || errDesc === void 0 ? void 0 : errDesc.set) === null || _a === void 0 ? void 0 : _a.call(source, handleBrowserErrorEvent);
        if (closeDesc === null || closeDesc === void 0 ? void 0 : closeDesc.set) { // WebSocket
            closeDesc.set.call(source, () => {
                var _a, _b;
                handleClose();
                (_a = closeDesc.set) === null || _a === void 0 ? void 0 : _a.call(source, null);
                (_b = errDesc === null || errDesc === void 0 ? void 0 : errDesc.set) === null || _b === void 0 ? void 0 : _b.call(source, null);
                cleanup === null || cleanup === void 0 ? void 0 : cleanup();
            });
        }
        else if (!(closeDesc === null || closeDesc === void 0 ? void 0 : closeDesc.set) && isFunction(source["close"])) { // EventSource
            // EventSource by default does not trigger close event, we need to
            // make sure when it calls the close() function, the iterator is
            // automatically closed.
            const es = source;
            const _close = es.close;
            Object.defineProperty(es, "close", {
                configurable: true,
                writable: true,
                value: function close() {
                    var _a;
                    _close.call(es);
                    handleClose();
                    Object.defineProperty(es, "close", {
                        configurable: true,
                        writable: true,
                        value: _close,
                    });
                    (_a = errDesc === null || errDesc === void 0 ? void 0 : errDesc.set) === null || _a === void 0 ? void 0 : _a.call(source, null);
                    cleanup === null || cleanup === void 0 ? void 0 : cleanup();
                }
            });
        }
    }
    else if (isFunction(source["send"]) && isFunction(source["close"])) {
        // non-standard WebSocket implementation or WebSocket-like object
        const ws = source;
        if (typeof ws.addEventListener === "function") {
            const msgListener = (ev) => {
                handleMessage(ev.data);
            };
            ws.addEventListener("message", msgListener);
            ws.addEventListener("error", handleBrowserErrorEvent);
            ws.addEventListener("close", () => {
                handleClose();
                ws.removeEventListener("message", msgListener);
                ws.removeEventListener("error", handleBrowserErrorEvent);
            });
        }
        else {
            ws.onmessage = (ev) => {
                handleMessage(ev.data);
            };
            ws.onerror = handleBrowserErrorEvent;
            ws.onclose = () => {
                handleClose();
                ws.onclose = null;
                ws.onerror = null;
                ws.onmessage = null;
            };
        }
    }
    else if (isFunction(source["addEventListener"])) { // EventTarget
        const target = source;
        const msgEvent = (eventMap === null || eventMap === void 0 ? void 0 : eventMap.message) || "message";
        const errEvent = (eventMap === null || eventMap === void 0 ? void 0 : eventMap.error) || "error";
        const closeEvent = (eventMap === null || eventMap === void 0 ? void 0 : eventMap.close) || "close";
        const msgListener = (ev) => {
            if (ev instanceof MessageEvent) {
                handleMessage(ev.data);
            }
        };
        target.addEventListener(msgEvent, msgListener);
        target.addEventListener(errEvent, handleBrowserErrorEvent);
        target.addEventListener(closeEvent, function closeListener() {
            handleClose();
            target.removeEventListener(closeEvent, closeListener);
            target.removeEventListener(msgEvent, msgListener);
            target.removeEventListener(errEvent, handleBrowserErrorEvent);
        });
    }
    else if (isFunction(source["on"])) { // EventEmitter
        const target = source;
        let dataEvent;
        let errEvent;
        let closeEvent;
        if (typeof process === "object" && source === process) {
            dataEvent = "message";
            errEvent = "uncaughtException";
            closeEvent = "exit";
        }
        else if ((isFunction(source["send"]) && isFunction(source["kill"])) || // child process
            (isFunction(source["postMessage"]) && isFunction(source["terminate"])) || // worker thread
            (isFunction(source["postMessage"]) && isFunction(source["close"])) // message port
        ) {
            dataEvent = "message";
            errEvent = "error";
            closeEvent = "exit";
        }
        else {
            dataEvent = (eventMap === null || eventMap === void 0 ? void 0 : eventMap.data) || "data";
            errEvent = (eventMap === null || eventMap === void 0 ? void 0 : eventMap.error) || "error";
            closeEvent = (eventMap === null || eventMap === void 0 ? void 0 : eventMap.close) || "close";
        }
        target.on(dataEvent, handleMessage);
        target.once(errEvent, handleClose);
        target.once(closeEvent, () => {
            handleClose();
            target.off(dataEvent, handleMessage);
            target.off(errEvent, handleClose);
        });
    }
    else {
        throw new TypeError("The  source cannot be converted to an async iterable object.");
    }
    return {
        [Symbol.asyncIterator]: channel[Symbol.asyncIterator].bind(channel),
    };
}

export { asAsyncIterable, resolveByteStream, resolveReadableStream, toAsyncIterable };
//# sourceMappingURL=util.js.map
