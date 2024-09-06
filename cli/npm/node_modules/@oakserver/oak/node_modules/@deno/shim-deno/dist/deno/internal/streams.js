"use strict";
/// <reference path="../stable/lib.deno.d.ts" />
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
var _BufferStreamReader_instances, _BufferStreamReader_stream, _BufferStreamReader_error, _BufferStreamReader_ended, _BufferStreamReader_pendingActions, _BufferStreamReader_runPendingActions, _StreamWriter_stream;
Object.defineProperty(exports, "__esModule", { value: true });
exports.StreamWriter = exports.BufferStreamReader = void 0;
class BufferStreamReader {
    constructor(stream) {
        _BufferStreamReader_instances.add(this);
        _BufferStreamReader_stream.set(this, void 0);
        _BufferStreamReader_error.set(this, void 0);
        _BufferStreamReader_ended.set(this, false);
        _BufferStreamReader_pendingActions.set(this, []);
        __classPrivateFieldSet(this, _BufferStreamReader_stream, stream, "f");
        __classPrivateFieldGet(this, _BufferStreamReader_stream, "f").pause();
        __classPrivateFieldGet(this, _BufferStreamReader_stream, "f").on("error", (error) => {
            __classPrivateFieldSet(this, _BufferStreamReader_error, error, "f");
            __classPrivateFieldGet(this, _BufferStreamReader_instances, "m", _BufferStreamReader_runPendingActions).call(this);
        });
        __classPrivateFieldGet(this, _BufferStreamReader_stream, "f").on("readable", () => {
            __classPrivateFieldGet(this, _BufferStreamReader_instances, "m", _BufferStreamReader_runPendingActions).call(this);
        });
        __classPrivateFieldGet(this, _BufferStreamReader_stream, "f").on("end", () => {
            __classPrivateFieldSet(this, _BufferStreamReader_ended, true, "f");
            __classPrivateFieldGet(this, _BufferStreamReader_instances, "m", _BufferStreamReader_runPendingActions).call(this);
        });
    }
    readAll() {
        return new Promise((resolve, reject) => {
            const chunks = [];
            const action = () => {
                if (__classPrivateFieldGet(this, _BufferStreamReader_error, "f")) {
                    reject(__classPrivateFieldGet(this, _BufferStreamReader_error, "f"));
                    return;
                }
                const buffer = __classPrivateFieldGet(this, _BufferStreamReader_stream, "f").read();
                if (buffer != null) {
                    chunks.push(buffer);
                    __classPrivateFieldGet(this, _BufferStreamReader_pendingActions, "f").push(action);
                }
                else if (__classPrivateFieldGet(this, _BufferStreamReader_ended, "f")) {
                    const result = Buffer.concat(chunks);
                    resolve(result);
                }
                else {
                    __classPrivateFieldGet(this, _BufferStreamReader_pendingActions, "f").push(action);
                }
            };
            action();
        });
    }
    read(p) {
        return new Promise((resolve, reject) => {
            const action = () => {
                if (__classPrivateFieldGet(this, _BufferStreamReader_error, "f")) {
                    reject(__classPrivateFieldGet(this, _BufferStreamReader_error, "f"));
                    return;
                }
                const readBuffer = __classPrivateFieldGet(this, _BufferStreamReader_stream, "f").read(p.byteLength);
                if (readBuffer && readBuffer.byteLength > 0) {
                    readBuffer.copy(p, 0, 0, readBuffer.byteLength);
                    resolve(readBuffer.byteLength);
                    return;
                }
                if (__classPrivateFieldGet(this, _BufferStreamReader_ended, "f")) {
                    resolve(null);
                }
                else {
                    __classPrivateFieldGet(this, _BufferStreamReader_pendingActions, "f").push(action);
                }
            };
            action();
        });
    }
}
exports.BufferStreamReader = BufferStreamReader;
_BufferStreamReader_stream = new WeakMap(), _BufferStreamReader_error = new WeakMap(), _BufferStreamReader_ended = new WeakMap(), _BufferStreamReader_pendingActions = new WeakMap(), _BufferStreamReader_instances = new WeakSet(), _BufferStreamReader_runPendingActions = function _BufferStreamReader_runPendingActions() {
    const errors = [];
    for (const action of __classPrivateFieldGet(this, _BufferStreamReader_pendingActions, "f").splice(0)) {
        try {
            action();
        }
        catch (err) {
            errors.push(err);
        }
    }
    if (errors.length > 0) {
        throw (errors.length > 1
            ? new globalThis.AggregateError(errors)
            : errors[0]);
    }
};
class StreamWriter {
    constructor(stream) {
        _StreamWriter_stream.set(this, void 0);
        __classPrivateFieldSet(this, _StreamWriter_stream, stream, "f");
    }
    write(p) {
        return new Promise((resolve, reject) => {
            __classPrivateFieldGet(this, _StreamWriter_stream, "f").write(p, (err) => {
                if (err) {
                    reject(err);
                }
                else {
                    resolve(p.byteLength);
                }
            });
        });
    }
}
exports.StreamWriter = StreamWriter;
_StreamWriter_stream = new WeakMap();
