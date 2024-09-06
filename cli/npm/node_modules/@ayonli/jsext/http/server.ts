import type { Server as HttpServer } from "node:http";
import type { Http2SecureServer } from "node:http2";
import type { serve, serveStatic } from "../http.ts";
import type { EventEndpoint } from "../sse.ts";
import { until } from "../async.ts";
import { isBun, isDeno, isNode } from "../env.ts";
import runtime, { env } from "../runtime.ts";
import { WebSocketConnection, WebSocketHandler, WebSocketServer } from "../ws.ts";
import { KVNamespace } from "../workerd/types.ts";
import { createRequestContext, createTimingFunctions, listenFetchEvent, withHeaders, patchTimingMetrics } from "./internal.ts";

export interface BunServer {
    fetch(request: Request | string): Response | Promise<Response>;
    ref(): void;
    requestIP(request: Request): { family: "IPv4" | "IPv6"; address: string; port: number; } | null;
    stop(closeActiveConnections?: boolean): void;
    unref(): void;
    upgrade<T = undefined>(
        request: Request,
        options?: {
            data?: T;
            headers?: HeadersInit;
        },
    ): boolean;

    readonly development: boolean;
    readonly hostname: string;
    readonly id: string;
    readonly pendingRequests: number;
    readonly pendingWebSockets: number;
    readonly port: number;
    readonly url: URL;
}

export interface FetchEvent extends Event {
    request: Request;
    respondWith(response: Response | Promise<Response>): void;
    waitUntil?(promise: Promise<unknown>): void;
    client?: {
        address: string;
    };
}

/**
 * Represents the network address of a connection peer.
 */
export interface NetAddress {
    family: "IPv4" | "IPv6";
    /**
     * The IP address of the remote peer.
     */
    address: string;
    /**
     * The port number of the remote peer, or `0` if it's not available.
     */
    port: number;
}

/**
 * Represents the context of an HTTP request. It provides additional information
 * about the request and allows for upgrading the connection to a WebSocket.
 */
export interface RequestContext {
    /**
     * The remote address of the client. This property may not be available in
     * worker environments (such as Cloudflare Workers) or when the server is
     * started via `deno serve`.
     */
    remoteAddress: NetAddress | null;
    /**
     * Creates an SSE (server-sent events) endpoint for sending events to the
     * client.
     */
    createEventEndpoint(): { events: EventEndpoint<Request>; response: Response; };
    /**
     * Upgrades the request to a WebSocket connection.
     */
    upgradeWebSocket(): { socket: WebSocketConnection; response: Response; };
    /**
     * Starts a timer that can be used to compute the duration of an operation
     * identified by a unique `name`. When the operation completes, call
     * `timeEnd()` with the same name to stop the timer.
     * 
     * This function is similar to the `console.time`, except it logs the
     * duration to the `Server-Timing` header of the response and will be
     * displayed in the browser's devtools.
     * 
     * Optionally, we can provide a `description` that will be used as the title
     * when displaying the timing metrics.
     * 
     * We could use a `total` label to measure the total time spent, which has
     * special meaning in the Google Chrome browser. However, it may not be
     * accurate since multiple operations can happen at the same time
     * concurrently.
     * 
     * @see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Server-Timing
     */
    time(name: string, description?: string): void;
    /**
     * Stops a timer that was previously started by calling `time()` with the
     * same `name`.
     */
    timeEnd(name: string): void;
    /**
     * Prolongs the request's lifetime until the promise is resolved. Only
     * available in workers environments such as Cloudflare Workers.
     */
    waitUntil?: ((promise: Promise<unknown>) => void) | undefined;
    /**
     * The bindings of the request, only available in Cloudflare Workers.
     */
    bindings?: {
        [x: string]: any;
        __STATIC_CONTENT?: KVNamespace;
    } | undefined;
}

/**
 * The handler for processing HTTP requests.
 */
export type RequestHandler = (request: Request, ctx: RequestContext) => Response | Promise<Response>;

/**
 * The handler for processing errors happened during processing HTTP requests.
 */
export type RequestErrorHandler = (error: unknown, request: Request, ctx: RequestContext) => Response | Promise<Response>;

/**
 * Options for serving HTTP requests, used by {@link serve}.
 */
export interface ServeOptions {
    /**
     * Instructs how the server should be deployed. `classic` means {@link serve}
     * will start the server itself (or use `addEventListener("fetch")` in
     * service workers), while `module` means using the {@link Server} instance
     * as an ES module with the syntax `export default serve({ ... })`.
     * 
     * NOTE: This option is only adjustable in Node.js, Deno, Bun and Cloudflare
     * Workers, in other environments, it will be ignored and will default to
     * `classic`. It is recommended to set this option to `module` in Cloudflare
     * Workers.
     * 
     * @default "classic"
     */
    type?: "classic" | "module";
    /**
     * The handler for processing HTTP requests.
     */
    fetch: RequestHandler;
    /**
     * The hostname to listen on. Default is `0.0.0.0`.
     * 
     * NOTE: This option is ignored in workers or when the `type` is `module`.
     */
    hostname?: string | undefined;
    /**
     * The port to listen on. If not set, the server will first try to use the
     * `8000` port, and if it's not available, it will use a random port.
     * 
     * NOTE: This option is ignored in workers or when the `type` is `module`.
     */
    port?: number | undefined;
    /**
     * The certificate key for serving HTTPS/HTTP2 requests.
     * 
     * NOTE: This option is ignored in workers or when the `type` is `module`.
     */
    key?: string | undefined;
    /**
     * The certificate for serving HTTPS/HTTP2 requests.
     * 
     * NOTE: This option is ignored in workers or when the `type` is `module`.
     */
    cert?: string | undefined;
    /**
     * The WebSocket handler for processing WebSocket connections. Normally this
     * options is not set and the WebSocket is handled per request inside the
     * `fetch` handler.
     */
    ws?: WebSocketHandler | undefined;
    /**
     * A listener that will be called when the `fetch` handler throws an error.
     * By default, the server will respond with a `500 Internal Server Error`
     * response, we can override this behavior by setting this option.
     */
    onError?: RequestErrorHandler | undefined;
    /**
     * A listener that will be called when the server starts listening. By
     * default, the server will log the address it's listening on, we can
     * override this behavior by setting this option.
     * 
     * NOTE: This option is ignored in workers or when the `type` is `module`.
     */
    onListen?: ((info: { hostname: string; port: number; }) => void) | undefined;
    /**
     * Extra headers to be sent with the response. These headers are only set
     * when they're not present.
     * 
     * By default, the server will set the `Server` header to the runtime name
     * and its version. We can set this option to override the default behavior,
     * or set it to `null` to disable the default headers.
     */
    headers?: HeadersInit | null | undefined;
}


/**
 * Options for serving static files, used by {@link serveStatic}.
 */
export interface ServeStaticOptions {
    /**
     * The file system directory to serve files from. If not set, the current
     * working directory will be used. This option is not available in
     * Cloudflare Workers, set `kv` instead.
     */
    fsDir?: string;
    /**
     * A KV namespace in Cloudflare Workers where the static files are stored.
     * This option is only needed in Cloudflare Workers, usually obtained from
     * the `__STATIC_CONTENT` binding.
     */
    kv?: KVNamespace;
    /**
     * The prefix that will be stripped from the URL pathname.
     */
    urlPrefix?: string;
    /**
     * Whether to list the directory entries when the URL pathname is a
     * directory. If not set, a 403 Forbidden response will be returned.
     */
    listDir?: boolean;
    /**
     * The maximum age in seconds for the "Cache-Control" header.
     */
    maxAge?: number;
    /**
     * Extra headers to be sent with the response.
     */
    headers?: HeadersInit;
}

const _hostname = Symbol.for("hostname");
const _port = Symbol.for("port");
const _http = Symbol.for("http");
const _controller = Symbol.for("controller");

/**
 * A unified HTTP server interface.
 */
export class Server {
    readonly type: "classic" | "module";
    private [_hostname] = "0.0.0.0";
    private [_port] = 0;
    private [_http]: Promise<HttpServer | Http2SecureServer | Deno.HttpServer | BunServer | null>;
    private [_controller]: AbortController | null = null;

    /**
     * A request handler for using the server instance as an ES module worker,
     * only available when the server type is `module`.
     */
    fetch?: ((request: Request, env?: any, ctx?: any) => Response | Promise<Response>);

    constructor(impl: () => Promise<{
        http: HttpServer | Http2SecureServer | Deno.HttpServer | BunServer | null;
        hostname: string;
        port: number;
        controller: AbortController | null;
    }>, options: Pick<ServeOptions, "type" | "fetch" | "onError" | "onListen" | "headers"> & {
        ws: WebSocketServer;
        secure?: boolean;
    }) {
        this.type = options.type ?? "classic";
        const { fetch: handle, ws, headers, secure } = options;

        const onError = options.onError ?? ((err) => {
            console.error(err);
            return new Response("Internal Server Error", {
                status: 500,
                statusText: "Internal Server Error",
            });
        });

        const defaultOnListen: ServeOptions["onListen"] = ({ hostname, port }) => {
            const _hostname = hostname === "0.0.0.0" ? "localhost" : hostname;
            const protocol = secure ? "https" : "http";
            console.log(`Server listening on ${protocol}://${_hostname}:${port}`);
        };
        const onListen = this.type === "classic"
            ? (options.onListen ?? defaultOnListen)
            : defaultOnListen;

        this[_http] = impl().then(({ http, hostname, port, controller }) => {
            this[_hostname] = hostname;
            this[_port] = port;
            this[_controller] = controller;

            if (http || isBun) {
                onListen({ hostname, port });
            }

            return http;
        });

        if (isDeno) {
            if (this.type === "classic") {
                delete this.fetch;
            } else {
                this.fetch = (req) => {
                    const { getTimers, time, timeEnd } = createTimingFunctions();
                    const ctx = createRequestContext(req, {
                        ws,
                        remoteAddress: null,
                        time,
                        timeEnd,
                    });
                    const _handle = withHeaders(handle, headers);
                    const _onError = withHeaders(onError, headers);

                    return _handle(req, ctx)
                        .then(res => patchTimingMetrics(res, getTimers()))
                        .catch(err => _onError(err, req, ctx));
                };
            }
        } else if (isBun) {
            if (this.type === "classic") {
                delete this.fetch;
            } else {
                this.fetch = (req, server: BunServer) => {
                    ws.bunBind(server);

                    const { getTimers, time, timeEnd } = createTimingFunctions();
                    const ctx = createRequestContext(req, {
                        ws,
                        remoteAddress: server.requestIP(req),
                        time,
                        timeEnd,
                    });
                    const _handle = withHeaders(handle, headers);
                    const _onError = withHeaders(onError, headers);

                    return _handle(req, ctx)
                        .then(res => patchTimingMetrics(res, getTimers()))
                        .catch(err => _onError(err, req, ctx));
                };

                Object.assign(this, {
                    // Bun specific properties
                    websocket: ws.bunListener,
                });
            }
        } else if (isNode) {
            if (this.type === "classic") {
                delete this.fetch;
            } else {
                this.fetch = (req, ctx: RequestContext) => {
                    const _handle = withHeaders(handle, headers);
                    const _onError = withHeaders(onError, headers);

                    return _handle(req, ctx)
                        .catch(err => _onError(err, req, ctx));
                };
            }
        } else if (typeof addEventListener === "function") {
            if (this.type === "classic") {
                let bindings: any;

                if (runtime().identity === "workerd") {
                    bindings = {};
                    Object.keys(globalThis).forEach((key) => {
                        if (/^[A-Z][A-Z0-9_]*$/.test(key)) {
                            // @ts-ignore
                            bindings[key] = globalThis[key];
                        }
                    });
                    env(bindings as object);
                }

                listenFetchEvent({ ws, fetch: handle, onError, headers, bindings });
            } else {
                this.fetch = (req, bindings, _ctx) => {
                    if (bindings && typeof bindings === "object" && !Array.isArray(bindings)) {
                        env(bindings as object);
                    }

                    const address = req.headers.get("cf-connecting-ip");
                    const { getTimers, time, timeEnd } = createTimingFunctions();
                    const ctx = createRequestContext(req, {
                        ws,
                        remoteAddress: address ? {
                            family: address.includes(":") ? "IPv6" : "IPv4",
                            address: address,
                            port: 0,
                        } : null,
                        time,
                        timeEnd,
                        waitUntil: _ctx.waitUntil?.bind(_ctx),
                        bindings,
                    });

                    const _handle = withHeaders(handle, headers);
                    const _onError = withHeaders(onError, headers);

                    return _handle(req, ctx)
                        .then(res => patchTimingMetrics(res, getTimers()))
                        .catch(err => _onError(err, req, ctx));
                };
            }
        }
    }

    /**
     * The hostname of which the server is listening on, only available after
     * the server is ready and the server type is `classic`.
     */
    get hostname(): string {
        return this[_hostname] || "";
    }

    /**
     * The port of which the server is listening on, only available after the
     * server is ready and the server type is `classic`.
     */
    get port(): number {
        return this[_port] || (isBun && this.type === "module" ? 3000 : 0);
    }

    /**
     * A promise that resolves when the server is ready to accept connections.
     */
    get ready(): Promise<this> {
        return this[_http].then(() => this);
    }

    /**
     * Closes the server and stops it from accepting new connections. By default,
     * this function will wait until all active connections to close before
     * shutting down the server. However, we can force the server to close all
     * active connections and shutdown immediately by setting the `force`
     * parameter to `true`.
     * 
     * NOTE: In Node.js, the `force` parameter is only available for HTTP
     * servers, it has no effect on HTTP2 servers.
     */
    async close(force = false): Promise<void> {
        const server = await this[_http] as any;

        if (!server)
            return;

        if (typeof server.stop === "function") {
            const _server = server as BunServer;

            _server.stop(force);
            if (!force) {
                await until(() => !_server.pendingRequests && !_server.pendingWebSockets);
            }
        } else if (typeof server.shutdown === "function") {
            const _server = server as Deno.HttpServer;

            if (force && this[_controller]) {
                this[_controller].abort();
            } else {
                _server.shutdown();
            }

            await _server.finished;
        } else if (typeof server.close === "function") {
            const _server = server as HttpServer | Http2SecureServer;

            await new Promise<void>((resolve, reject) => {
                _server.close((err) => err ? reject(err) : resolve());

                if (force && "closeAllConnections" in _server) {
                    _server.closeAllConnections();
                }
            });
        }
    }

    /**
     * Opposite of `unref()`, calling `ref()` on a previously `unref`ed server
     * will _not_ let the program exit if it's the only server left (the default
     * behavior). If the server is `ref`ed calling `ref()` again will have no
     * effect.
     */
    ref(): void {
        this[_http].then(server => server?.ref?.());
    }

    /**
     * Calling `unref()` on a server will allow the program to exit if this is
     * the only active server in the event system. If the server is already
     * `unref`ed calling`unref()` again will have no effect.
     */
    unref(): void {
        this[_http].then(server => server?.unref?.());
    }
}
