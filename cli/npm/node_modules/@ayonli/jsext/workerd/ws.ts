import { createCloseEvent, createErrorEvent } from "../event.ts";
import type { BunServer } from "../http/server.ts";
import { ServerOptions, WebSocketConnection, WebSocketHandler, WebSocketLike } from "../ws/base.ts";
import type { IncomingMessage } from "node:http";

export type { ServerOptions, WebSocketConnection, WebSocketHandler, WebSocketLike };

const _handler = Symbol.for("handler");
const _clients = Symbol.for("clients");

declare var WebSocketPair: {
    new(): [WebSocket, WebSocket & { accept: () => void; }];
};

export class WebSocketServer {
    protected idleTimeout: number;
    protected perMessageDeflate: boolean;
    protected [_handler]: WebSocketHandler | undefined;
    protected [_clients]: Map<Request, WebSocketConnection> = new Map();

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

    upgrade(request: Request): { socket: WebSocketConnection; response: Response; };
    upgrade(request: IncomingMessage): { socket: WebSocketConnection; };
    upgrade(request: Request | IncomingMessage): { socket: WebSocketConnection; response: Response; } {
        if ("socket" in request) {
            throw new TypeError("Expected a Request instance");
        }

        const upgradeHeader = request.headers.get("Upgrade");
        if (!upgradeHeader || upgradeHeader !== "websocket") {
            throw new TypeError("Expected Upgrade: websocket");
        } else if (typeof WebSocketPair !== "function") {
            throw new Error("WebSocket is not supported in this environment");
        }

        const handler = this[_handler];
        const clients = this[_clients];

        const [client, server] = Object.values(new WebSocketPair()) as [WebSocket, WebSocket & {
            accept: () => void;
        }];
        const socket = new WebSocketConnection(new Promise<WebSocketLike>(resolve => {
            server.accept();
            server.addEventListener("message", ev => {
                if (typeof ev.data === "string") {
                    socket.dispatchEvent(new MessageEvent("message", {
                        data: ev.data,
                    }));
                } else if (ev.data instanceof ArrayBuffer) {
                    socket.dispatchEvent(new MessageEvent("message", {
                        data: new Uint8Array(ev.data),
                    }));
                } else {
                    (ev.data as Blob).arrayBuffer().then(buffer => {
                        socket.dispatchEvent(new MessageEvent("message", {
                            data: new Uint8Array(buffer),
                        }));
                    }).catch(() => { });
                }
            });
            server.addEventListener("close", ev => {
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
            });

            if (server.readyState === 1) {
                resolve(server);
            } else {
                server.addEventListener("open", () => {
                    resolve(server);
                });
            }
        }));

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
                // @ts-ignore
                webSocket: client,
            }),
        };
    }

    bunBind(server: BunServer): void {
        void server;
    }

    get bunListener(): {
        idleTimeout: number;
        perMessageDeflate: boolean;
        message: (ws: any, message: string | ArrayBuffer | Uint8Array) => void;
        open: (ws: any) => void;
        error: (ws: any, error: Error) => void;
        close: (ws: any, code: number, reason: string) => void;
    } {
        return {
            idleTimeout: this.idleTimeout,
            perMessageDeflate: this.perMessageDeflate,
            message: () => void 0,
            open: () => void 0,
            error: () => void 0,
            close: () => void 0,
        };
    }
}
