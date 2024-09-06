"use strict";
///<reference path="../lib.deno.d.ts" />
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
var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var _a, _b;
var _c, _d;
var _FsFile_closed, _FsFile_readableStream, _FsFile_writableStream;
Object.defineProperty(exports, "__esModule", { value: true });
exports.File = exports.FsFile = void 0;
const fs = __importStar(require("node:fs"));
const stream = __importStar(require("node:stream"));
const fstat_js_1 = require("../functions/fstat.js");
const fstatSync_js_1 = require("../functions/fstatSync.js");
const ftruncate_js_1 = require("../functions/ftruncate.js");
const ftruncateSync_js_1 = require("../functions/ftruncateSync.js");
const fdatasync_js_1 = require("../functions/fdatasync.js");
const fdatasyncSync_js_1 = require("../functions/fdatasyncSync.js");
const read_js_1 = require("../functions/read.js");
const readSync_js_1 = require("../functions/readSync.js");
const write_js_1 = require("../functions/write.js");
const writeSync_js_1 = require("../functions/writeSync.js");
(_a = (_c = Symbol).dispose) !== null && _a !== void 0 ? _a : (_c.dispose = Symbol("Symbol.dispose"));
(_b = (_d = Symbol).asyncDispose) !== null && _b !== void 0 ? _b : (_d.asyncDispose = Symbol("Symbol.asyncDispose"));
class FsFile {
    constructor(rid) {
        this.rid = rid;
        _FsFile_closed.set(this, false);
        _FsFile_readableStream.set(this, void 0);
        _FsFile_writableStream.set(this, void 0);
    }
    [(_FsFile_closed = new WeakMap(), _FsFile_readableStream = new WeakMap(), _FsFile_writableStream = new WeakMap(), Symbol.dispose)]() {
        if (!__classPrivateFieldGet(this, _FsFile_closed, "f")) {
            this.close();
        }
    }
    async write(p) {
        return await (0, write_js_1.write)(this.rid, p);
    }
    writeSync(p) {
        return (0, writeSync_js_1.writeSync)(this.rid, p);
    }
    async truncate(len) {
        await (0, ftruncate_js_1.ftruncate)(this.rid, len);
    }
    truncateSync(len) {
        return (0, ftruncateSync_js_1.ftruncateSync)(this.rid, len);
    }
    read(p) {
        return (0, read_js_1.read)(this.rid, p);
    }
    readSync(p) {
        return (0, readSync_js_1.readSync)(this.rid, p);
    }
    seek(_offset, _whence) {
        throw new Error("Method not implemented.");
    }
    seekSync(_offset, _whence) {
        throw new Error("Method not implemented.");
    }
    async stat() {
        return await (0, fstat_js_1.fstat)(this.rid);
    }
    statSync() {
        return (0, fstatSync_js_1.fstatSync)(this.rid);
    }
    sync() {
        throw new Error("Method not implemented.");
    }
    syncSync() {
        throw new Error("Method not implemented.");
    }
    syncData() {
        return (0, fdatasync_js_1.fdatasync)(this.rid);
    }
    syncDataSync() {
        return (0, fdatasyncSync_js_1.fdatasyncSync)(this.rid);
    }
    utime(_atime, _mtime) {
        throw new Error("Method not implemented.");
    }
    utimeSync(_atime, _mtime) {
        throw new Error("Method not implemented.");
    }
    close() {
        __classPrivateFieldSet(this, _FsFile_closed, true, "f");
        fs.closeSync(this.rid);
    }
    get readable() {
        if (__classPrivateFieldGet(this, _FsFile_readableStream, "f") == null) {
            const nodeStream = fs.createReadStream(null, {
                fd: this.rid,
                autoClose: false,
            });
            __classPrivateFieldSet(this, _FsFile_readableStream, stream.Readable.toWeb(nodeStream), "f");
        }
        return __classPrivateFieldGet(this, _FsFile_readableStream, "f");
    }
    get writable() {
        if (__classPrivateFieldGet(this, _FsFile_writableStream, "f") == null) {
            const nodeStream = fs.createWriteStream(null, {
                fd: this.rid,
                autoClose: false,
            });
            __classPrivateFieldSet(this, _FsFile_writableStream, stream.Writable.toWeb(nodeStream), "f");
        }
        return __classPrivateFieldGet(this, _FsFile_writableStream, "f");
    }
}
exports.FsFile = FsFile;
const File = FsFile;
exports.File = File;
