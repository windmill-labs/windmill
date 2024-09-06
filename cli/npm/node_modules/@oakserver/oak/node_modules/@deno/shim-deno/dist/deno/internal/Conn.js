"use strict";
///<reference path="../stable/lib.deno.d.ts" />
var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var _Conn_socket;
Object.defineProperty(exports, "__esModule", { value: true });
exports.TlsConn = exports.Conn = void 0;
const net_1 = require("net");
const FsFile_js_1 = require("../stable/classes/FsFile.js");
class Conn extends FsFile_js_1.FsFile {
    constructor(rid, localAddr, remoteAddr, socket) {
        super(rid);
        this.rid = rid;
        this.localAddr = localAddr;
        this.remoteAddr = remoteAddr;
        _Conn_socket.set(this, void 0);
        __classPrivateFieldSet(this, _Conn_socket, socket || new net_1.Socket({ fd: rid }), "f");
    }
    [(_Conn_socket = new WeakMap(), Symbol.dispose)]() {
        this.close();
    }
    async closeWrite() {
        await new Promise((resolve) => __classPrivateFieldGet(this, _Conn_socket, "f").end(resolve));
    }
    setNoDelay(enable) {
        __classPrivateFieldGet(this, _Conn_socket, "f").setNoDelay(enable);
    }
    setKeepAlive(enable) {
        __classPrivateFieldGet(this, _Conn_socket, "f").setKeepAlive(enable);
    }
    ref() {
        __classPrivateFieldGet(this, _Conn_socket, "f").ref();
    }
    unref() {
        __classPrivateFieldGet(this, _Conn_socket, "f").unref();
    }
}
exports.Conn = Conn;
class TlsConn extends Conn {
    handshake() {
        console.warn("@deno/shim-deno: Handshake is not supported.");
        return Promise.resolve({
            alpnProtocol: null,
        });
    }
}
exports.TlsConn = TlsConn;
