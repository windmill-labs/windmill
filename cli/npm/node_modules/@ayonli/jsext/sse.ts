/**
 * This module provides tools for working with server-sent events.
 * 
 * The {@link EventEndpoint} is used to handle SSE requests on the server
 * and send messages to the client, while the {@link EventSource} and the
 * {@link EventConsumer} are used to read and process messages sent by the
 * server.
 * 
 * Despite their names, these classes can be used in both the browser/client
 * and the server environments.
 * 
 * NOTE: This module depends on the Fetch API and Web Streams API, in Node.js,
 * it requires Node.js v18.0 or above.
 * 
 * @module
 * @experimental
 */
import "./external/event-target-polyfill/index.ts";
import type { IncomingMessage, ServerResponse } from "node:http";
import type { Http2ServerRequest, Http2ServerResponse } from "node:http2";
import type { Constructor } from "./types.ts";
import { createCloseEvent, createErrorEvent } from "./event.ts";
import { isBun, isDeno } from "./env.ts";
import runtime, { customInspect } from "./runtime.ts";
import _try from "./try.ts";

if (typeof MessageEvent !== "function" || runtime().identity === "workerd") {
    // Worker environments does not implement or only partially implement the MessageEvent, 
    // we need to implement it ourselves.
    globalThis.MessageEvent = class MessageEvent<T = any> extends Event implements globalThis.MessageEvent<T> {
        readonly data: T = undefined as T;
        readonly lastEventId: string = "";
        readonly origin: string = "";
        readonly ports: ReadonlyArray<MessagePort> = [];
        readonly source: MessageEventSource | null = null;

        constructor(type: string, eventInitDict: MessageEventInit<T> | undefined = undefined) {
            super(type, eventInitDict);

            if (eventInitDict) {
                this.data = eventInitDict.data as T;
                this.lastEventId = eventInitDict.lastEventId ?? "";
                this.origin = eventInitDict.origin ?? "";
                this.ports = eventInitDict.ports ?? [];
            }
        }

        initMessageEvent(
            type: string,
            bubbles = false,
            cancelable = false,
            data = null,
            origin = "",
            lastEventId = "",
            source: MessageEventSource | null = null,
            ports: MessagePort[] = []
        ): void {
            this.initEvent(type, bubbles ?? false, cancelable ?? false);
            Object.assign(this, { data, origin, lastEventId, source, ports });
        }
    };
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

const SSEMarkClosed = new Set<string>();

const _closed = Symbol.for("closed");
const _request = Symbol.for("request");
const _response = Symbol.for("response");
const _writer = Symbol.for("writer");
const _lastEventId = Symbol.for("lastEventId");
const _reconnectionTime = Symbol.for("reconnectionTime");
const _retry = Symbol.for("retry");
const _timer = Symbol.for("timer");
const _controller = Symbol.for("controller");
const _onopen = Symbol.for("onopen");
const _onerror = Symbol.for("onerror");
const _onmessage = Symbol.for("onmessage");

function setReadonly<T>(obj: any, name: string | symbol, value: T) {
    Object.defineProperty(obj, name, {
        configurable: true,
        enumerable: false,
        writable: false,
        value,
    });
}

function getReadonly<T>(obj: any, name: string | symbol): T | undefined {
    return Object.getOwnPropertyDescriptor(obj, name)?.value;
}

function fixStringTag(ctor: Constructor): void {
    setReadonly(ctor.prototype, Symbol.toStringTag, ctor.name);
}

/**
 * The options for the {@link EventEndpoint} constructor.
 */
export interface EventEndpointOptions {
    /**
     * The time in milliseconds that instructs the client to wait before
     * reconnecting.
     */
    reconnectionTime?: number;
}

/**
 * An SSE (server-sent events) implementation that can be used to send messages
 * to the client. This implementation is based on the `EventTarget` interface
 * and conforms the web standard.
 * 
 * **Events:**
 * 
 * - `close` - Dispatched when the connection is closed.
 * 
 * @example
 * ```ts
 * // with Web APIs
 * import { EventEndpoint } from "@ayonli/jsext/sse";
 * 
 * export default {
 *     async fetch(req: Request) {
 *         const events = new EventEndpoint(req);
 * 
 *         events.addEventListener("close", (ev) => {
 *             console.log(`The connection is closed, reason: ${ev.reason}`);
 *         });
 * 
 *         setTimeout(() => {
 *             events.dispatchEvent(new MessageEvent("my-event", {
 *                 data: "Hello, World!",
 *                 lastEventId: "1",
 *             }));
 *         }, 1_000);
 * 
 *         return events.response!;
 *     }
 * }
 * ```
 * 
 * @example
 * ```ts
 * // with Node.js APIs
 * import * as http from "node:http";
 * import { EventEndpoint } from "@ayonli/jsext/sse";
 * 
 * const server = http.createServer((req, res) => {
 *     const events = new EventEndpoint(req, res);
 * 
 *     events.addEventListener("close", (ev) => {
 *         console.log(`The connection is closed, reason: ${ev.reason}`);
 *     });
 * 
 *     setTimeout(() => {
 *         events.dispatchEvent(new MessageEvent("my-event", {
 *             data: "Hello, World!",
 *             lastEventId: "1",
 *         }));
 *     }, 1_000);
 * });
 * 
 * server.listen(3000);
 * ```
 */
export class EventEndpoint<T extends Request | IncomingMessage | Http2ServerRequest = Request | IncomingMessage | Http2ServerRequest> extends EventTarget {
    private [_writer]: WritableStreamDefaultWriter<Uint8Array>;
    private [_response]: Response | null;
    private [_lastEventId]: string;
    private [_reconnectionTime]: number;
    private [_closed]: boolean;

    constructor(request: T, options?: EventEndpointOptions);
    constructor(
        request: T,
        response: ServerResponse | Http2ServerResponse,
        options?: EventEndpointOptions
    );
    constructor(
        request: Request | IncomingMessage | Http2ServerRequest,
        ...args: any[]
    ) {
        super();

        const isNodeRequest = "socket" in request && "socket" in args[0];
        let options: EventEndpointOptions;

        if (isNodeRequest) {
            const req = request as IncomingMessage | Http2ServerRequest;
            this[_lastEventId] = String(req.headers["last-event-id"] ?? "");
            options = args[1] ?? {};
        } else {
            this[_lastEventId] = (request as Request).headers.get("Last-Event-ID") ?? "";
            options = args[0] ?? {};
        }

        this[_reconnectionTime] = options.reconnectionTime ?? 0;
        this[_closed] = this[_lastEventId]
            ? SSEMarkClosed.has(this[_lastEventId])
            : false;
        const resInit: ResponseInit = {
            status: this.closed ? 204 : 200,
            statusText: this.closed ? "No Content" : "OK",
            headers: {
                "Content-Type": "text/event-stream",
                "Cache-Control": "no-cache",
                "Connection": "keep-alive",
                "Transfer-Encoding": "chunked",
            },
        };

        const _this = this;

        if (isNodeRequest) {
            this[_response] = null;
            const res = args[0] as ServerResponse | Http2ServerResponse;
            const writable = new WritableStream({
                write(chunk) {
                    (res as ServerResponse).write(chunk);
                },
                close() {
                    res.closed || res.end();
                    _this[_closed] = true;
                    _this.dispatchEvent(createCloseEvent("close", { wasClean: true }));
                },
                abort(err) {
                    res.closed || res.destroy(err);
                },
            });

            this[_writer] = writable.getWriter();
            res.once("close", () => {
                this[_writer].close().catch(() => { });
            }).once("error", (err) => {
                this[_writer].abort(err).catch(() => { });
            });

            for (const [name, value] of Object.entries(resInit.headers!)) {
                // Use `setHeader` to set headers instead of passing them to `writeHead`,
                // it seems in Deno, the headers are not written to the response if they
                // are passed to `writeHead`.
                res.setHeader(name, value);
            }

            res.writeHead(resInit.status!, resInit.statusText!);
            (res as ServerResponse).write(new Uint8Array(0));
        } else {
            const { writable, readable } = new TransformStream<Uint8Array, Uint8Array>();
            const reader = readable.getReader();

            const _readable = new ReadableStream<Uint8Array>({
                async start(controller) {
                    if (isBun) {
                        // In Bun, the response will not be sent to the client
                        // until the first non-empty chunk is written. May be a
                        // bug, but we need to work around it now.
                        controller.enqueue(encoder.encode(":ok\n\n"));
                    } else {
                        controller.enqueue(new Uint8Array(0));
                    }
                },
                async pull(controller) {
                    while (true) {
                        const { done, value } = await reader.read();
                        if (done) {
                            try { controller.close(); } catch { }
                            _this[_closed] = true;
                            _this.dispatchEvent(createCloseEvent("close", { wasClean: true }));
                            break;
                        }

                        controller.enqueue(value);
                    }
                },
                async cancel(reason) {
                    await reader.cancel(reason);
                }
            });

            this[_writer] = writable.getWriter();
            this[_response] = new Response(this.closed ? null : _readable, resInit);
        }

        this.closed && this.close();
    }

    /**
     * The last event ID that the server has sent.
     */
    get lastEventId(): string {
        return this[_lastEventId];
    }

    /**
     * Indicates whether the connection has been closed.
     */
    get closed(): boolean {
        return this[_closed];
    }

    /**
     * The response that will be sent to the client, only available when the
     * instance is created with the `Request` API.
     */
    get response(): T extends Request ? Response : null {
        return this[_response] as any;
    }

    /**
     * Adds an event listener that will be called when the connection is closed.
     */
    override addEventListener(
        type: "close",
        listener: (this: EventEndpoint<T>, ev: CloseEvent) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    override addEventListener(
        type: string,
        listener: EventListenerOrEventListenerObject | null,
        options?: boolean | AddEventListenerOptions
    ): void;
    override addEventListener(
        event: string,
        listener: any,
        options: boolean | AddEventListenerOptions | undefined = undefined
    ): void {
        return super.addEventListener(event, listener, options);
    }

    override removeEventListener(
        type: "close",
        listener: (this: EventEndpoint<T>, ev: CloseEvent) => void,
        options?: boolean | EventListenerOptions | undefined
    ): void;
    override removeEventListener(
        type: string,
        listener: EventListenerOrEventListenerObject | null,
        options?: boolean | EventListenerOptions | undefined
    ): void;
    override removeEventListener(
        type: string,
        listener: any,
        options: boolean | EventListenerOptions | undefined = undefined
    ): void {
        return super.removeEventListener(type, listener, options);
    }

    /**
     * Dispatches an message event that will be sent to the client.
     */
    override dispatchEvent(event: MessageEvent<string>): boolean;
    override dispatchEvent(event: CloseEvent | ErrorEvent | Event): boolean;
    override dispatchEvent(event: MessageEvent | CloseEvent | ErrorEvent | Event): boolean {
        if (event instanceof MessageEvent) {
            if (event.type === "message") {
                this.send(event.data, event.lastEventId).catch(() => { });
            } else {
                this.sendEvent(event.type, event.data, event.lastEventId)
                    .catch(() => { });
            }

            return !event.cancelable || !event.defaultPrevented;
        } else {
            return super.dispatchEvent(event);
        }
    }

    private buildMessage(data: string, options: {
        id?: string | undefined;
        event?: string | undefined;
    } = {}): Uint8Array {
        let message = "";

        if (options.id) {
            this[_lastEventId] = options.id;
            message += `id: ${options.id}\n`;
        }

        if (options.event) {
            message += `event: ${options.event}\n`;
        }

        if (this[_reconnectionTime]) {
            message += `retry: ${this[_reconnectionTime]}\n`;
        }

        message += data.split(/\r\n|\n/).map((line) => `data: ${line}\n`).join("");
        message += "\n";

        return encoder.encode(message);
    }

    /**
     * Sends a message to the client.
     * 
     * The client (`EventSource` or {@link EventConsumer}) will receive the
     * message as a `MessageEvent`, which can be listened to using the
     * `message` event.
     * 
     * @param eventId If specified, the client will remember the value as the
     * last event ID and will send it back to the server when reconnecting.
     */
    async send(data: string, eventId: string | undefined = undefined): Promise<void> {
        await this[_writer].write(this.buildMessage(data, { id: eventId }));
    }

    /**
     * Sends a custom event to the client.
     * 
     * The client (`EventSource` or {@link EventConsumer}) will receive the
     * event as a `MessageEvent`, which can be listened to using the custom
     * event name.
     * 
     * @param eventId If specified, the client will remember the value as the
     * last event ID and will send it back to the server when reconnecting.
     */
    async sendEvent(
        event: string,
        data: string,
        eventId: string | undefined = undefined
    ): Promise<void> {
        await this[_writer].write(this.buildMessage(data, { id: eventId, event }));
    }

    /**
     * Closes the connection.
     * 
     * By default, when the connection is closed, the client will try to
     * reconnect after a certain period of time, which is specified by the
     * `reconnectionTime` option when creating the instance.
     * 
     * However, if the `noReconnect` parameter is set, this method will mark
     * the client as closed based on the last event ID. When the client
     * reconnects, the server will send a `204 No Content` response to the
     * client to instruct it to terminate the connection.
     * 
     * It is important to note that the server depends on the last event ID to
     * identify the client for this purpose, so the server must send a globally
     * unique `lastEventId` to the client when sending messages.
     */
    close(noReconnect = false): void {
        this[_writer].close().catch(() => { }).finally(() => {
            this[_closed] = true;

            if (this.lastEventId) {
                if (!SSEMarkClosed.has(this.lastEventId)) {
                    noReconnect && SSEMarkClosed.add(this.lastEventId);
                } else {
                    SSEMarkClosed.delete(this.lastEventId);
                }
            }
        });
    }
}
fixStringTag(EventEndpoint);

async function readAndProcessResponse(
    response: Response,
    handlers: {
        onId: (id: string) => void;
        onRetry: (time: number) => void;
        onData: (data: string, event: string) => void;
        onError: (error: unknown) => void;
        onEnd: () => void;
    }
): Promise<void> {
    const reader = response.body!.getReader();
    let buffer: string = "";

    try {
        while (true) {
            const { done, value } = await reader.read();

            if (done) {
                handlers.onEnd();
                break;
            }

            buffer += decoder.decode(value);

            const chunks = buffer.split(/\r\n\r\n|\n\n/);

            if (chunks.length === 1) {
                continue;
            } else {
                buffer = chunks.pop()!;
            }

            for (const chunk of chunks) {
                const lines = chunk.split(/\r\n|\n/);
                let data = "";
                let type = "message";
                let isMessage = false;

                for (const line of lines) {
                    if (line.startsWith("data:") || line === "data") {
                        let value = line.slice(5);

                        if (value[0] === " ") {
                            value = value.slice(1);
                        }

                        if (data) {
                            data += "\n" + value;
                        } else {
                            data = value;
                        }

                        isMessage = true;
                    } else if (line.startsWith("event:") || line === "event") {
                        type = line.slice(6).trim();
                        isMessage = true;
                    } else if (line.startsWith("id:") || line === "id") {
                        handlers.onId(line.slice(3).trim());
                        isMessage = true;
                    } else if (line.startsWith("retry:")) {
                        const time = parseInt(line.slice(6).trim());
                        if (!isNaN(time) && time >= 0) {
                            handlers.onRetry(time);
                            isMessage = true;
                        }
                    }
                }

                if (isMessage) {
                    handlers.onData(data, type || "message");
                }
            }
        }
    } catch (error) {
        handlers.onError(error);
    }
}

/**
 * @deprecated Use {@link EventEndpoint} instead.
 */
export const SSE = EventEndpoint;

/**
 * @deprecated Use {@link EventEndpointOptions} instead.
 */
export type SSEOptions = EventEndpointOptions;

/**
 * This is an implementation of the
 * [EventSource](https://developer.mozilla.org/en-US/docs/Web/API/EventSource)
 * API that serves as a polyfill in environments that do not have native support,
 * such as Node.js.
 * 
 * NOTE: This API depends on the Fetch API and Web Streams API, in Node.js, it
 * requires Node.js v18.0 or above.
 * 
 * @example
 * ```ts
 * import { EventSource } from "@ayonli/jsext/sse";
 * 
 * globalThis.EventSource ??= EventSource;
 * 
 * const events = new EventSource("http://localhost:3000");
 * 
 * events.addEventListener("open", () => {
 *     console.log("The connection is open.");
 * });
 * 
 * events.addEventListener("error", (ev) => {
 *     console.error("An error occurred:", ev.error);
 * });
 * 
 * events.addEventListener("message", (ev) => {
 *     console.log("Received message from the server:", ev.data);
 * });
 * 
 * events.addEventListener("my-event", (ev) => {
 *     console.log("Received custom event from the server:", ev.data);
 * });
 * ```
 */
export class EventSource extends EventTarget {
    private [_controller]: AbortController = new AbortController();
    private [_request]: Request | null = null;
    private [_lastEventId]: string = "";
    private [_reconnectionTime]: number = 0;
    private [_retry]: number = 0;
    private [_timer]: number | NodeJS.Timeout | null = null;
    private [_onopen]: ((this: EventSource, ev: Event) => any) | null = null;
    private [_onmessage]: ((this: EventSource, ev: MessageEvent<string>) => any) | null = null;
    private [_onerror]: ((this: EventSource, ev: ErrorEvent) => any) | null = null;

    readonly CONNECTING = EventSource.CONNECTING;
    readonly OPEN = EventSource.OPEN;
    readonly CLOSED = EventSource.CLOSED;

    static CONNECTING = 0 as const;
    static OPEN = 1 as const;
    static CLOSED = 2 as const;

    get url(): string {
        return getReadonly(this, "url") ?? "";
    }

    get withCredentials(): boolean {
        return getReadonly(this, "withCredentials") ?? false;
    }

    get readyState(): number {
        return getReadonly(this, "readyState") ?? this.CONNECTING;
    }

    get onopen(): ((this: EventSource, ev: Event) => any) | null {
        return this[_onopen] ?? null;
    }
    set onopen(value: ((this: EventSource, ev: Event) => any) | null) {
        this[_onopen] = value;
    }

    get onmessage(): ((this: EventSource, ev: MessageEvent<string>) => any) | null {
        return this[_onmessage] ?? null;
    }
    set onmessage(value: ((this: EventSource, ev: MessageEvent<string>) => any) | null) {
        this[_onmessage] = value;
    }

    get onerror(): ((this: EventSource, ev: ErrorEvent) => any) | null {
        return this[_onerror] ?? null;
    }
    set onerror(value: ((this: EventSource, ev: ErrorEvent) => any) | null) {
        this[_onerror] = value;
    }

    constructor(url: string | URL, options: EventSourceInit = {}) {
        super();
        url = typeof url === "object" ? url.href : new URL(url,
            typeof location === "object" ? location.href : undefined).href;

        setReadonly(this, "url", url);
        setReadonly(this, "withCredentials", options.withCredentials ?? false);
        setReadonly(this, "readyState", this.CONNECTING);

        setReadonly(this, "CONNECTING", EventSource.CONNECTING);
        setReadonly(this, "OPEN", EventSource.OPEN);
        setReadonly(this, "CLOSED", EventSource.CLOSED);

        this.connect().catch(() => { });
    }

    private async connect() {
        if (this.readyState === this.CLOSED) {
            return;
        }

        setReadonly(this, "readyState", this.CONNECTING);
        const headers: HeadersInit = {
            "Accept": "text/event-stream",
        };

        if (this[_lastEventId]) {
            headers["Last-Event-ID"] = this[_lastEventId];
        }

        this[_request] = new Request(this.url, {
            headers,
            credentials: this.withCredentials ? "include" : "same-origin",
            cache: "no-store",
            signal: this[_controller].signal,
        });

        const [err, res] = await _try(fetch(this[_request]));

        if (err || res?.type === "error") {
            const event = createErrorEvent("error", {
                error: err || new Error(`Failed to fetch '${this.url}'`),
            });
            this.dispatchEvent(event);
            this.onerror?.call(this, event);
            this.tryReconnect();
            return;
        } else if (res.status === 204) { // No more data, close the connection
            setReadonly(this, "readyState", this.CLOSED);
            return;
        } else if (res.status !== 200) {
            setReadonly(this, "readyState", this.CLOSED);
            const event = createErrorEvent("error", {
                error: new TypeError(`The server responded with status ${res.status}.`),
            });
            this.dispatchEvent(event);
            this.onerror?.call(this, event);
            return;
        } else if (!res.headers.get("Content-Type")?.startsWith("text/event-stream")) {
            setReadonly(this, "readyState", this.CLOSED);
            const event = createErrorEvent("error", {
                error: new TypeError("The response is not an event stream."),
            });
            this.dispatchEvent(event);
            this.onerror?.call(this, event);
            return;
        } else if (!res.body) {
            setReadonly(this, "readyState", this.CLOSED);
            const event = createErrorEvent("error", {
                error: new TypeError("The response does not have a body."),
            });
            this.dispatchEvent(event);
            this.onerror?.call(this, event);
            return;
        }

        setReadonly(this, "readyState", this.OPEN);
        this[_retry] = 0;
        const event = new Event("open");
        this.dispatchEvent(event);
        this.onopen?.call(this, event);

        const origin = new URL(res.url || this.url).origin;
        await readAndProcessResponse(res, {
            onId: (id) => {
                this[_lastEventId] = id;
            },
            onRetry: (time) => {
                this[_reconnectionTime] = time;
            },
            onData: (data, event) => {
                const _event = new MessageEvent(event, {
                    lastEventId: this[_lastEventId],
                    data,
                    origin,
                });
                this.dispatchEvent(_event);
                this.onmessage?.call(this, _event);
            },
            onError: (error) => {
                if (this.readyState !== this.CLOSED) {
                    const event = createErrorEvent("error", { error });
                    this.dispatchEvent(event);
                    this.onerror?.call(this, event);
                    this.tryReconnect();
                }
            },
            onEnd: () => {
                if (this.readyState !== this.CLOSED) {
                    const event = createErrorEvent("error", {
                        error: new Error("The connection is interrupted."),
                    });
                    this.dispatchEvent(event);
                    this.onerror?.call(this, event);
                    this.tryReconnect();
                }
            },
        });
    }

    private tryReconnect() {
        if (this[_timer]) {
            clearTimeout(this[_timer]);
            this[_timer] = null;
        }

        this[_timer] = setTimeout(() => {
            this.connect().catch(() => { });
        }, this[_reconnectionTime] || 1000 * Math.min(30, Math.pow(2, this[_retry]++)));
    }

    /**
     * Closes the connection.
     */
    close(): void {
        if (this[_timer]) {
            clearTimeout(this[_timer]);
            this[_timer] = null;
        }
        setReadonly(this, "readyState", this.CLOSED);
        this[_controller].abort();
    }

    /**
     * Adds an event listener that will be called when the connection is open.
     */
    override addEventListener(
        type: "open",
        listener: (this: EventSource, ev: Event) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when the connection is
     * interrupted.
     */
    override addEventListener(
        type: "error",
        listener: (this: EventSource, ev: ErrorEvent) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when a message with the
     * default event type is received.
     */
    override addEventListener(
        type: "message",
        listener: (this: EventSource, ev: MessageEvent<string>) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when a message with a custom
     * event type is received.
     */
    override addEventListener(
        type: string,
        listener: (this: EventSource, event: MessageEvent<string>) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    override addEventListener(
        type: string,
        listener: EventListenerOrEventListenerObject | null,
        options?: boolean | AddEventListenerOptions
    ): void;
    override addEventListener(
        event: string,
        listener: any,
        options: boolean | AddEventListenerOptions | undefined = undefined
    ): void {
        return super.addEventListener(event, listener, options);
    }

    override removeEventListener(
        type: "open",
        listener: (this: EventSource, ev: Event) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: "error",
        listener: (this: EventSource, ev: ErrorEvent) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: "message",
        listener: (this: EventSource, ev: MessageEvent<string>) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: string,
        listener: (this: EventSource, event: MessageEvent<string>) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: string,
        listener: EventListenerOrEventListenerObject | null,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: string,
        listener: any,
        options: boolean | EventListenerOptions | undefined = undefined
    ): void {
        return super.removeEventListener(type, listener, options);
    }

    [customInspect](): object | string {
        const _this = this;

        if (isDeno) {
            return "EventSource " + Deno.inspect({
                readyState: _this.readyState,
                url: _this.url,
                withCredentials: _this.withCredentials,
                onopen: _this.onopen,
                onmessage: _this.onmessage,
                onerror: _this.onerror,
            }, { colors: true });
        } else {
            return new class EventSource {
                readyState = _this.readyState;
                url = _this.url;
                withCredentials = _this.withCredentials;
                onopen = _this.onopen;
                onmessage = _this.onmessage;
                onerror = _this.onerror;
            };
        }
    }
}
fixStringTag(EventSource);

/**
 * Unlike the {@link EventSource} API, which takes a URL and only supports GET
 * request, the {@link EventConsumer} API accepts a `Response` object and reads
 * the messages from its body, the response can be generated from any type of
 * request, usually returned from the `fetch` function.
 * 
 * This API doesn't support active closure and reconnection, however, we can
 * use `AbortController` in the request to terminate the connection, and add
 * a event listener to the close event and reestablish the connection manually.
 * 
 * **Events:**
 * 
 * - `error` - Dispatched when an error occurs, such as network failure. After
 *   this event is dispatched, the connection will be closed and the `close`
 *   event will be dispatched.
 * - `close` - Dispatched when the connection is closed. If the connection is
 *   closed due to some error, the `error` event will be dispatched before this
 *   event, and the close event will have the `wasClean` set to `false`, and the
 *   `reason` property contains the error message, if any.
 * - `message` - Dispatched when a message with the default event type is
 *   received.
 * - custom events - Dispatched when a message with a custom event type is
 *   received.
 * 
 * NOTE: This API depends on the Fetch API and Web Streams API, in Node.js, it
 * requires Node.js v18.0 or above.
 * 
 * @example
 * ```ts
 * import { EventConsumer } from "@ayonli/jsext/sse";
 * 
 * const response = await fetch("http://localhost:3000", {
 *     method: "POST",
 *     headers: {
 *         "Accept": "text/event-stream",
 *     },
 * });
 * const events = new EventConsumer(response);
 * 
 * events.addEventListener("close", (ev) => {
 *     console.log(`The connection is closed, reason: ${ev.reason}`);
 * 
 *     if (!ev.wasClean) {
 *         // perhaps to reestablish the connection
 *     }
 * });
 * 
 * events.addEventListener("my-event", (ev) => {
 *     console.log(`Received message from the server: ${ev.data}`);
 * });
 * ```
 */
export class EventConsumer extends EventTarget {
    private [_lastEventId]: string = "";
    private [_reconnectionTime]: number = 0;
    private [_closed] = false;

    constructor(response: Response) {
        super();

        if (!response.body) {
            throw new TypeError("The response does not have a body.");
        } else if (response.bodyUsed) {
            throw new TypeError("The response body has already been used.");
        } else if (response.body.locked) {
            throw new TypeError("The response body is locked.");
        } else if (!response.headers.get("Content-Type")?.startsWith("text/event-stream")) {
            throw new TypeError("The response is not an event stream.");
        }

        const origin = response.url ? new URL(response.url).origin : "";
        readAndProcessResponse(response, {
            onId: (id) => {
                this[_lastEventId] = id;
            },
            onRetry: (time) => {
                this[_reconnectionTime] = time;
            },
            onData: (data, event) => {
                this.dispatchEvent(new MessageEvent(event, {
                    lastEventId: this[_lastEventId],
                    data,
                    origin,
                }));
            },
            onError: (error) => {
                this[_closed] = true;
                this.dispatchEvent(createErrorEvent("error", { error }));
                this.dispatchEvent(createCloseEvent("close", {
                    reason: error instanceof Error ? error.message : String(error),
                    wasClean: false,
                }));
            },
            onEnd: () => {
                this[_closed] = true;
                this.dispatchEvent(createCloseEvent("close", { wasClean: true }));
            },
        }).catch(() => { });
    }

    /**
     * The last event ID that the server has sent.
     */
    get lastEventId(): string {
        return this[_lastEventId];
    }

    /**
     * The time in milliseconds that instructs the client to wait before
     * reconnecting.
     * 
     * NOTE: The {@link EventConsumer} API does not support auto-reconnection,
     * this value is only used when we want to reestablish the connection
     * manually.
     */
    get retry(): number {
        return this[_reconnectionTime];
    }

    /**
     * Indicates whether the connection has been closed.
     */
    get closed(): boolean {
        return this[_closed];
    }

    /**
     * Adds an event listener that will be called when the connection is
     * interrupted. After this event is dispatched, the connection will be
     * closed and the `close` event will be dispatched.
     */
    override addEventListener(
        type: "error",
        listener: (this: EventConsumer, ev: ErrorEvent) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when the connection is closed.
     * If the connection is closed due to some error, the `error` event will be
     * dispatched before this event, and the close event will have the `wasClean`
     * set to `false`, and the `reason` property contains the error message, if
     * any.
     */
    override addEventListener(
        type: "close",
        listener: (this: EventConsumer, ev: CloseEvent) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when a message with the
     * default event type is received.
     */
    override addEventListener(
        type: "message",
        listener: (this: EventConsumer, ev: MessageEvent<string>) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when a message with a custom
     * event type is received.
     */
    override addEventListener(
        type: string,
        listener: (this: EventConsumer, event: MessageEvent<string>) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    override addEventListener(
        type: string,
        listener: EventListenerOrEventListenerObject | null,
        options?: boolean | AddEventListenerOptions
    ): void;
    override addEventListener(
        event: string,
        listener: any,
        options: boolean | AddEventListenerOptions | undefined = undefined
    ): void {
        return super.addEventListener(event, listener, options);
    }

    override removeEventListener(
        type: "error",
        listener: (this: EventConsumer, ev: ErrorEvent) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: "close",
        listener: (this: EventConsumer, ev: CloseEvent) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: "message",
        listener: (this: EventConsumer, ev: MessageEvent<string>) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: string,
        listener: (this: EventConsumer, event: MessageEvent<string>) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: string,
        listener: EventListenerOrEventListenerObject | null,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: string,
        listener: any,
        options: boolean | EventListenerOptions | undefined = undefined
    ): void {
        return super.removeEventListener(type, listener, options);
    }
}
fixStringTag(EventConsumer);

/**
 * @deprecated Use {@link EventConsumer} instead.
 */
export const EventClient = EventConsumer;
