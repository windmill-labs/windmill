import { until } from '../async.js';
import { isBun, isDeno, isNode } from '../env.js';
import runtime, { env } from '../runtime.js';
import { createRequestContext, withHeaders, patchTimingMetrics, listenFetchEvent, createTimingFunctions } from './internal.js';

var _a, _b, _c;
const _hostname = Symbol.for("hostname");
const _port = Symbol.for("port");
const _http = Symbol.for("http");
const _controller = Symbol.for("controller");
/**
 * A unified HTTP server interface.
 */
class Server {
    constructor(impl, options) {
        var _d, _e, _f;
        this[_a] = "0.0.0.0";
        this[_b] = 0;
        this[_c] = null;
        this.type = (_d = options.type) !== null && _d !== void 0 ? _d : "classic";
        const { fetch: handle, ws, headers, secure } = options;
        const onError = (_e = options.onError) !== null && _e !== void 0 ? _e : ((err) => {
            console.error(err);
            return new Response("Internal Server Error", {
                status: 500,
                statusText: "Internal Server Error",
            });
        });
        const defaultOnListen = ({ hostname, port }) => {
            const _hostname = hostname === "0.0.0.0" ? "localhost" : hostname;
            const protocol = secure ? "https" : "http";
            console.log(`Server listening on ${protocol}://${_hostname}:${port}`);
        };
        const onListen = this.type === "classic"
            ? ((_f = options.onListen) !== null && _f !== void 0 ? _f : defaultOnListen)
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
            }
            else {
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
        }
        else if (isBun) {
            if (this.type === "classic") {
                delete this.fetch;
            }
            else {
                this.fetch = (req, server) => {
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
        }
        else if (isNode) {
            if (this.type === "classic") {
                delete this.fetch;
            }
            else {
                this.fetch = (req, ctx) => {
                    const _handle = withHeaders(handle, headers);
                    const _onError = withHeaders(onError, headers);
                    return _handle(req, ctx)
                        .catch(err => _onError(err, req, ctx));
                };
            }
        }
        else if (typeof addEventListener === "function") {
            if (this.type === "classic") {
                let bindings;
                if (runtime().identity === "workerd") {
                    bindings = {};
                    Object.keys(globalThis).forEach((key) => {
                        if (/^[A-Z][A-Z0-9_]*$/.test(key)) {
                            // @ts-ignore
                            bindings[key] = globalThis[key];
                        }
                    });
                    env(bindings);
                }
                listenFetchEvent({ ws, fetch: handle, onError, headers, bindings });
            }
            else {
                this.fetch = (req, bindings, _ctx) => {
                    var _d;
                    if (bindings && typeof bindings === "object" && !Array.isArray(bindings)) {
                        env(bindings);
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
                        waitUntil: (_d = _ctx.waitUntil) === null || _d === void 0 ? void 0 : _d.bind(_ctx),
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
    get hostname() {
        return this[_hostname] || "";
    }
    /**
     * The port of which the server is listening on, only available after the
     * server is ready and the server type is `classic`.
     */
    get port() {
        return this[_port] || (isBun && this.type === "module" ? 3000 : 0);
    }
    /**
     * A promise that resolves when the server is ready to accept connections.
     */
    get ready() {
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
    async close(force = false) {
        const server = await this[_http];
        if (!server)
            return;
        if (typeof server.stop === "function") {
            const _server = server;
            _server.stop(force);
            if (!force) {
                await until(() => !_server.pendingRequests && !_server.pendingWebSockets);
            }
        }
        else if (typeof server.shutdown === "function") {
            const _server = server;
            if (force && this[_controller]) {
                this[_controller].abort();
            }
            else {
                _server.shutdown();
            }
            await _server.finished;
        }
        else if (typeof server.close === "function") {
            const _server = server;
            await new Promise((resolve, reject) => {
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
    ref() {
        this[_http].then(server => { var _d; return (_d = server === null || server === void 0 ? void 0 : server.ref) === null || _d === void 0 ? void 0 : _d.call(server); });
    }
    /**
     * Calling `unref()` on a server will allow the program to exit if this is
     * the only active server in the event system. If the server is already
     * `unref`ed calling`unref()` again will have no effect.
     */
    unref() {
        this[_http].then(server => { var _d; return (_d = server === null || server === void 0 ? void 0 : server.unref) === null || _d === void 0 ? void 0 : _d.call(server); });
    }
}
_a = _hostname, _b = _port, _c = _controller;

export { Server };
//# sourceMappingURL=server.js.map
