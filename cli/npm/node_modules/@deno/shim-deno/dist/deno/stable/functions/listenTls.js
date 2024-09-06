"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.listenTls = void 0;
const tls_1 = require("tls");
const Conn_js_1 = require("../../internal/Conn.js");
const Listener_js_1 = require("../../internal/Listener.js");
const readTextFileSync_js_1 = require("./readTextFileSync.js");
async function* _listen(server, waitFor) {
    await waitFor;
    while (server.listening) {
        yield new Promise((resolve) => server.once("secureConnection", (socket) => {
            socket.on("error", (err) => console.error(err));
            // @ts-expect-error undocumented socket._handle property
            const rid = socket._handle.fd;
            const localAddr = {
                // cannot be undefined while socket is connected
                hostname: socket.localAddress,
                port: socket.localPort,
                transport: "tcp",
            };
            const remoteAddr = {
                // cannot be undefined while socket is connected
                hostname: socket.remoteAddress,
                port: socket.remotePort,
                transport: "tcp",
            };
            resolve(new Conn_js_1.TlsConn(rid, localAddr, remoteAddr));
        }));
    }
}
const listenTls = function listen({ port, hostname = "0.0.0.0", transport = "tcp", certFile, keyFile }) {
    if (transport !== "tcp") {
        throw new Error("Deno.listen is only implemented for transport: tcp");
    }
    const [cert, key] = [certFile, keyFile].map((f) => f == null ? undefined : (0, readTextFileSync_js_1.readTextFileSync)(f));
    const server = (0, tls_1.createServer)({ cert, key });
    const waitFor = new Promise((resolve) => 
    // server._handle.fd is assigned immediately on .listen()
    server.listen(port, hostname, resolve));
    // @ts-expect-error undocumented socket._handle property
    const listener = new Listener_js_1.Listener(server._handle.fd, {
        hostname,
        port,
        transport: "tcp",
    }, _listen(server, waitFor));
    return listener;
};
exports.listenTls = listenTls;
