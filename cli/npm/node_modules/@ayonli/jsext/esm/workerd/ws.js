import { createErrorEvent, createCloseEvent } from '../event.js';
import { WebSocketConnection } from '../ws/base.js';

var _a;
const _handler = Symbol.for("handler");
const _clients = Symbol.for("clients");
class WebSocketServer {
    constructor(...args) {
        var _b, _c, _d;
        this[_a] = new Map();
        if (args.length === 2) {
            this.idleTimeout = ((_b = args[0]) === null || _b === void 0 ? void 0 : _b.idleTimeout) || 30;
            this.perMessageDeflate = (_d = (_c = args[0]) === null || _c === void 0 ? void 0 : _c.perMessageDeflate) !== null && _d !== void 0 ? _d : false;
            this[_handler] = args[1];
        }
        else {
            this.idleTimeout = 30;
            this.perMessageDeflate = false;
            this[_handler] = args[0];
        }
    }
    upgrade(request) {
        if ("socket" in request) {
            throw new TypeError("Expected a Request instance");
        }
        const upgradeHeader = request.headers.get("Upgrade");
        if (!upgradeHeader || upgradeHeader !== "websocket") {
            throw new TypeError("Expected Upgrade: websocket");
        }
        else if (typeof WebSocketPair !== "function") {
            throw new Error("WebSocket is not supported in this environment");
        }
        const handler = this[_handler];
        const clients = this[_clients];
        const [client, server] = Object.values(new WebSocketPair());
        const socket = new WebSocketConnection(new Promise(resolve => {
            server.accept();
            server.addEventListener("message", ev => {
                if (typeof ev.data === "string") {
                    socket.dispatchEvent(new MessageEvent("message", {
                        data: ev.data,
                    }));
                }
                else if (ev.data instanceof ArrayBuffer) {
                    socket.dispatchEvent(new MessageEvent("message", {
                        data: new Uint8Array(ev.data),
                    }));
                }
                else {
                    ev.data.arrayBuffer().then(buffer => {
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
            }
            else {
                server.addEventListener("open", () => {
                    resolve(server);
                });
            }
        }));
        socket.ready.then(() => {
            clients.set(request, socket);
            handler === null || handler === void 0 ? void 0 : handler.call(this, socket);
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
    bunBind(server) {
    }
    get bunListener() {
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
_a = _clients;

export { WebSocketServer };
//# sourceMappingURL=ws.js.map
