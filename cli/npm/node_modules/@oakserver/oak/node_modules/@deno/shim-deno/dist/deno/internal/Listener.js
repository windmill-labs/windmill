"use strict";
///<reference path="../stable/lib.deno.d.ts" />
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
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
var _Listener_listener;
Object.defineProperty(exports, "__esModule", { value: true });
exports.Listener = void 0;
const close_js_1 = require("../stable/functions/close.js");
const errors = __importStar(require("../stable/variables/errors.js"));
class Listener {
    constructor(rid, addr, listener) {
        this.rid = rid;
        this.addr = addr;
        _Listener_listener.set(this, void 0);
        __classPrivateFieldSet(this, _Listener_listener, listener, "f");
    }
    [(_Listener_listener = new WeakMap(), Symbol.dispose)]() {
        this.close();
    }
    async accept() {
        if (!__classPrivateFieldGet(this, _Listener_listener, "f")) {
            throw new errors.BadResource("Listener not initialised");
        }
        const result = await __classPrivateFieldGet(this, _Listener_listener, "f").next();
        if (result.done) {
            throw new errors.BadResource("Server not listening");
        }
        return result.value;
    }
    async next() {
        let conn;
        try {
            conn = await this.accept();
        }
        catch (error) {
            if (error instanceof errors.BadResource) {
                return { value: undefined, done: true };
            }
            throw error;
        }
        return { value: conn, done: false };
    }
    return(value) {
        this.close();
        return Promise.resolve({ value, done: true });
    }
    close() {
        (0, close_js_1.close)(this.rid);
    }
    ref() {
        throw new Error("Not implemented");
    }
    unref() {
        throw new Error("Not implemented");
    }
    [Symbol.asyncIterator]() {
        return this;
    }
}
exports.Listener = Listener;
