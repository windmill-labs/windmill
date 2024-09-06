/**
 * This module provides a unified WebSocket server interface for Node.js, Deno,
 * Bun and Cloudflare Workers. This module is based on the `EventTarget`
 * interface and conforms the web standard.
 * 
 * **IMPORTANT**: The {@link WebSocketConnection} interface is an abstraction of
 * the WebSocket on the server side, it's design is not consistent with the
 * {@link WebSocket} API in the browser. For example, when receiving binary data,
 * the `data` property is always a `Uint8Array` object, which is different from
 * the `Blob` object (or `ArrayBuffer`) in the browser. In the future, we may
 * provide a more consistent API, be aware of this when using this module.
 * @module
 * @experimental
 */

import { AsyncTask, asyncTask } from "./async.ts";
import { concat, text } from "./bytes.ts";
import { createCloseEvent, createErrorEvent } from "./event.ts";
import runtime from "./runtime.ts";
import type { BunServer } from "./http/server.ts";
import { ServerOptions, WebSocketConnection, WebSocketHandler, WebSocketLike } from "./ws/base.ts";
import type { WebSocketServer as WsServer } from "ws";
import type { IncomingMessage } from "node:http";

export type { ServerOptions, WebSocketConnection, WebSocketHandler, WebSocketLike };

const _errored = Symbol.for("errored");
const _handler = Symbol.for("handler");
const _clients = Symbol.for("clients");
const _httpServer = Symbol.for("httpServer");
const _wsServer = Symbol.for("wsServer");
const _connTasks = Symbol.for("connTasks");

/**
 * A unified WebSocket server interface for Node.js, Deno, Bun and Cloudflare
 * Workers.
 * 
 * There are two ways to handle WebSocket connections:
 * 
 * 1. By passing a listener function to the constructor, handle all connections
 *    in a central place.
 * 2. By using the `socket` object returned from the `upgrade` method to handle
 *    each connection in a more flexible way. However, at this stage, the
 *    connection may not be ready yet, and we need to listen the `open` event
 *    or wait for the `ready` promise (deprecated) to resolve before we can
 *    start sending messages.
 * 
 * The `socket` object is an async iterable object, which can be used in the
 * `for await...of` loop to read messages with backpressure support.
 * 
 * @example
 * ```ts
 * // centralized connection handler
 * import { WebSocketServer } from "@ayonli/jsext/ws";
 * 
 * const wsServer = new WebSocketServer(socket => {
 *     console.log("WebSocket connection established.");
 * 
 *     socket.addEventListener("message", (event) => {
 *         socket.send("received: " + event.data);
 *     });
 * 
 *     socket.addEventListener("error", (event) => {
 *         console.error("WebSocket connection error:", event.error);
 *     });
 * 
 *     socket.addEventListener("close", (event) => {
 *         console.log(`WebSocket connection closed, reason: ${event.reason}, code: ${event.code}`);
 *     });
 * });
 * 
 * // Node.js
 * import * as http from "node:http";
 * const httpServer = http.createServer(req => {
 *      wsServer.upgrade(req);
 * });
 * httpServer.listen(3000);
 * 
 * // Node.js (withWeb)
 * import { withWeb } from "@ayonli/jsext/http";
 * const httpServer2 = http.createServer(withWeb(req => {
 *      const { response } = wsServer.upgrade(req);
 *      return response;
 * }));
 * httpServer2.listen(3001);
 * 
 * // Bun
 * const bunServer = Bun.serve({
 *     fetch(req) {
 *         const { response } = wsServer.upgrade(req);
 *         return response;
 *     },
 *     websocket: wsServer.bunListener,
 * });
 * wsServer.bunBind(bunServer);
 * 
 * // Deno
 * Deno.serve(req => {
 *     const { response } = wsServer.upgrade(req);
 *     return response;
 * });
 * 
 * // Cloudflare Workers
 * export default {
 *     fetch(req) {
 *         const { response } = wsServer.upgrade(req);
 *         return response;
 *     },
 * };
 * ```
 * 
 * @example
 * ```ts
 * // per-request connection handler (Deno example)
 * import { WebSocketServer } from "@ayonli/jsext/ws";
 * 
 * const wsServer = new WebSocketServer();
 * 
 * Deno.serve(req => {
 *     const { socket, response } = wsServer.upgrade(req);
 * 
 *     socket.addEventListener("open", () => {
 *         console.log("WebSocket connection established.");
 *     });
 * 
 *     socket.addEventListener("message", (event) => {
 *         socket.send("received: " + event.data);
 *     });
 * 
 *     socket.addEventListener("error", (event) => {
 *         console.error("WebSocket connection error:", event.error);
 *     });
 * 
 *     socket.addEventListener("close", (event) => {
 *         console.log(`WebSocket connection closed, reason: ${event.reason}, code: ${event.code}`);
 *     });
 * 
 *     // The response should be returned immediately, otherwise the web socket
 *     // will not be ready.
 *     return response;
 * });
 * ```
 * 
 * @example
 * ```ts
 * // async iterable
 * const wsServer = new WebSocketServer(async socket => {
 *     console.log("WebSocket connection established.");
 * 
 *     try {
 *         for await (const message of socket) {
 *             socket.send("received: " + message);
 *         }
 *     } catch (error) {
 *         console.error("WebSocket connection error:", error);
 *     }
 * 
 *     console.log("WebSocket connection closed");
 * });
 * 
 * Deno.serve(req => {
 *     const { response } = wsServer.upgrade(req);
 *     return response;
 * });
 * ```
 */
export class WebSocketServer {
    protected idleTimeout: number;
    protected perMessageDeflate: boolean;
    protected [_handler]: WebSocketHandler | undefined;
    protected [_clients]: Map<Request | IncomingMessage, WebSocketConnection> = new Map();
    private [_httpServer]: BunServer | undefined = undefined;
    private [_wsServer]: Promise<WsServer> | null = null;
    private [_connTasks]: Map<Request, AsyncTask<WebSocketLike>> = new Map();

    constructor(handler?: WebSocketHandler | undefined);
    constructor(options: ServerOptions, handler: WebSocketHandler);
    constructor(...args: any[]) {
        if (args.length === 2) {
            this.idleTimeout = args[0]?.idleTimeout || 30;
            this.perMessageDeflate = args[0]?.perMessageDeflate ?? false;
            this[_handler] = args[1];
        } else {
            this.idleTimeout = 30;
            this.perMessageDeflate = false;
            this[_handler] = args[0];
        }
    }

    /**
     * Upgrades the request to a WebSocket connection in Deno, Bun and Cloudflare
     * Workers.
     * 
     * This function can also be used in Node.js if the HTTP request listener is
     * created using the `withWeb` function from `@ayonli/jsext/http` module.
     * 
     * NOTE: This function fails if the request is not a WebSocket upgrade request.
     */
    upgrade(request: Request): { socket: WebSocketConnection; response: Response; };
    /**
     * Upgrades the request to a WebSocket connection in Node.js.
     * 
     * NOTE: This function fails if the request is not a WebSocket upgrade request.
     */
    upgrade(request: IncomingMessage): { socket: WebSocketConnection; };
    upgrade(request: Request | IncomingMessage,): {
        socket: WebSocketConnection;
        response?: Response;
    } {
        const upgradeHeader = "socket" in request
            ? request.headers["upgrade"]
            : request.headers.get("Upgrade");
        if (!upgradeHeader || upgradeHeader !== "websocket") {
            throw new TypeError("Expected Upgrade: websocket");
        }

        const handler = this[_handler];
        const clients = this[_clients];
        const { identity } = runtime();

        if (identity === "deno") {
            if ("socket" in request) {
                throw new TypeError("Node.js support is not implemented outside Node.js runtime.");
            }

            const { socket: ws, response } = Deno.upgradeWebSocket(request, {
                idleTimeout: this.idleTimeout,
            });
            const socket = new WebSocketConnection(new Promise((resolve) => {
                ws.binaryType = "arraybuffer";
                ws.onmessage = (ev) => {
                    if (typeof ev.data === "string") {
                        socket.dispatchEvent(new MessageEvent("message", {
                            data: ev.data,
                        }));
                    } else {
                        socket.dispatchEvent(new MessageEvent("message", {
                            data: new Uint8Array(ev.data as ArrayBuffer),
                        }));
                    }
                };
                ws.onclose = (ev) => {
                    if (!ev.wasClean) {
                        socket.dispatchEvent(createErrorEvent("error", {
                            error: new Error(`WebSocket connection closed: ${ev.reason} (${ev.code})`),
                        }));
                    }

                    clients.delete(request);
                    socket.dispatchEvent(createCloseEvent("close", {
                        code: ev.code,
                        reason: ev.reason,
                        wasClean: ev.wasClean,
                    }));
                };

                if (ws.readyState === 1) {
                    resolve(ws);
                } else {
                    ws.onopen = () => {
                        resolve(ws);
                    };
                }
            }));

            socket.ready.then(() => {
                clients.set(request, socket);
                handler?.call(this, socket);
                socket.dispatchEvent(new Event("open"));
            });

            return { socket, response };
        } else if (identity === "bun") {
            if ("socket" in request) {
                throw new TypeError("Node.js support is not implemented outside Node.js runtime.");
            }

            const server = this[_httpServer];
            if (!server) {
                throw new Error("WebSocket server is not bound to a Bun server instance.");
            }

            const task = asyncTask<WebSocketLike>();
            this[_connTasks].set(request, task);

            const ok: boolean = server.upgrade(request, { data: { request } });
            if (!ok) {
                throw new Error("Failed to upgrade to WebSocket");
            }

            const socket = new WebSocketConnection(task);

            socket.ready.then(() => {
                clients.set(request, socket);
                handler?.call(this, socket);
                socket.dispatchEvent(new Event("open"));
            });

            return {
                socket,
                response: new Response(null, {
                    status: 101,
                    statusText: "Switching Protocols",
                    headers: new Headers({
                        "Upgrade": "websocket",
                        "Connection": "Upgrade",
                    }),
                }),
            };
        } else if (identity === "node") {
            const isNodeRequest = "socket" in request;

            if (!isNodeRequest && Reflect.has(request, Symbol.for("incomingMessage"))) {
                request = Reflect.get(request, Symbol.for("incomingMessage"));
            }

            if (!("socket" in request)) {
                throw new TypeError("Expected an instance of http.IncomingMessage");
            }

            const { socket } = request;
            const upgradeHeader = request.headers.upgrade;

            if (!upgradeHeader || upgradeHeader !== "websocket") {
                throw new TypeError("Expected Upgrade: websocket");
            }

            const handler = this[_handler];
            const clients = this[_clients];

            if (!this[_wsServer]) {
                this[_wsServer] = import("ws").then(({ WebSocketServer: WsServer }) => {
                    return new WsServer({
                        noServer: true,
                        perMessageDeflate: this.perMessageDeflate,
                    });
                });
            }

            const task = this[_wsServer].then(wsServer => new Promise<WebSocketLike>((resolve) => {
                wsServer.handleUpgrade(request as IncomingMessage, socket, Buffer.alloc(0), (ws) => {
                    ws.on("message", (data, isBinary) => {
                        data = Array.isArray(data) ? concat(...data) : data;
                        let event: MessageEvent<string | Uint8Array>;

                        if (typeof data === "string") {
                            event = new MessageEvent("message", { data });
                        } else {
                            if (isBinary) {
                                event = new MessageEvent("message", {
                                    data: new Uint8Array(data),
                                });
                            } else {
                                const bytes = data instanceof ArrayBuffer
                                    ? new Uint8Array(data)
                                    : data;
                                event = new MessageEvent("message", {
                                    data: text(bytes),
                                });
                            }
                        }

                        client.dispatchEvent(event);
                    });
                    ws.on("error", error => {
                        Object.assign(ws, { [_errored]: true });
                        client.dispatchEvent(createErrorEvent("error", { error }));
                    });
                    ws.on("close", (code, reason) => {
                        clients.delete(request as IncomingMessage);
                        client.dispatchEvent(createCloseEvent("close", {
                            code,
                            reason: reason?.toString("utf8") ?? "",
                            wasClean: Reflect.get(ws, _errored) !== false,
                        }));
                    });

                    resolve(ws as unknown as WebSocketLike);
                });
            }));
            const client = new WebSocketConnection(task);

            client.ready.then(() => {
                clients.set(request as IncomingMessage, client);
                handler?.call(this, client);
                client.dispatchEvent(new Event("open"));
            });

            if (!isNodeRequest && typeof Response === "function") {
                const response = new Response(null, {
                    status: 200,
                    statusText: "Switching Protocols",
                    headers: new Headers({
                        "Upgrade": "websocket",
                        "Connection": "Upgrade",
                    }),
                });

                // HACK: Node.js currently does not support setting the
                // status code to outside the range of 200 to 599. This
                // is a workaround to set the status code to 101.
                Object.defineProperty(response, "status", {
                    configurable: true,
                    value: 101,
                });

                return { socket: client, response };
            } else {
                return { socket: client };
            }
        } else {
            throw new TypeError("Unsupported runtime");
        }
    }

    /**
     * Used in Bun, to bind the WebSocket server to the Bun server instance.
     */
    bunBind(server: BunServer): void {
        this[_httpServer] = server;
    }

    /**
     * A WebSocket listener for `Bun.serve()`.
     */
    get bunListener(): {
        idleTimeout: number;
        perMessageDeflate: boolean;
        message: (ws: any, message: string | ArrayBuffer | Uint8Array) => void;
        open: (ws: any) => void;
        error: (ws: any, error: Error) => void;
        close: (ws: any, code: number, reason: string) => void;
    } {
        const clients = this[_clients];
        const connTasks = this[_connTasks];
        type ServerWebSocket = WebSocket & { data: { request: Request; }; };

        return {
            idleTimeout: this.idleTimeout,
            perMessageDeflate: this.perMessageDeflate,
            open: async (ws: ServerWebSocket) => {
                const { request } = ws.data;
                const task = connTasks.get(request);

                if (task) {
                    connTasks.delete(request);
                    task.resolve(ws);
                }
            },
            message: (ws: ServerWebSocket, msg: string | ArrayBuffer | Uint8Array) => {
                const { request } = ws.data;
                const client = clients.get(request);

                if (client) {
                    if (typeof msg === "string") {
                        client.dispatchEvent(new MessageEvent("message", {
                            data: msg,
                        }));
                    } else {
                        client.dispatchEvent(new MessageEvent("message", {
                            data: new Uint8Array(msg),
                        }));
                    }
                }
            },
            error: (ws: ServerWebSocket, error: Error) => {
                Object.assign(ws, { [_errored]: true });
                const { request } = ws.data;
                const client = clients.get(request);
                client && client.dispatchEvent(createErrorEvent("error", { error }));
            },
            close: (ws: ServerWebSocket, code: number, reason: string) => {
                const { request } = ws.data;
                const client = clients.get(request);

                if (client) {
                    clients.delete(request);
                    client.dispatchEvent(createCloseEvent("close", {
                        code,
                        reason,
                        wasClean: Reflect.get(ws, _errored) !== true,
                    }));
                }
            },
        };
    }

    /**
     * An iterator that yields all connected WebSocket clients, can be used to
     * broadcast messages to all clients.
     */
    get clients(): IterableIterator<WebSocketConnection> {
        return this[_clients].values();
    }
}
