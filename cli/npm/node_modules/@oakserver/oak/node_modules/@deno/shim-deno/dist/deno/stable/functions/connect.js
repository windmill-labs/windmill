"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.connect = void 0;
const net_1 = require("net");
const Conn_js_1 = require("../../internal/Conn.js");
const connect = function connect(options) {
    if (options.transport === "unix") {
        throw new Error("Unstable UnixConnectOptions is not implemented");
    }
    const { transport = "tcp", hostname = "127.0.0.1", port } = options;
    if (transport !== "tcp") {
        throw new Error("Deno.connect is only implemented for transport: tcp");
    }
    const socket = (0, net_1.createConnection)({ port, host: hostname });
    socket.on("error", (err) => console.error(err));
    return new Promise((resolve) => {
        socket.once("connect", () => {
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
            resolve(new Conn_js_1.Conn(rid, localAddr, remoteAddr, socket));
        });
    });
};
exports.connect = connect;
