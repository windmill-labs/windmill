"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.connectTls = void 0;
const tls_1 = require("tls");
const Conn_js_1 = require("../../internal/Conn.js");
const readTextFile_js_1 = require("./readTextFile.js");
const connectTls = async function connectTls({ port, hostname = "127.0.0.1", certFile }) {
    const cert = certFile && await (0, readTextFile_js_1.readTextFile)(certFile);
    const socket = (0, tls_1.connect)({ port, host: hostname, cert });
    return new Promise((resolve) => {
        socket.on("connect", () => {
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
            resolve(new Conn_js_1.TlsConn(rid, localAddr, remoteAddr, socket));
        });
    });
};
exports.connectTls = connectTls;
