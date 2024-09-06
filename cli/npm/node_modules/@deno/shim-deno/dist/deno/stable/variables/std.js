"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.stderr = exports.stdout = exports.stdin = void 0;
const node_stream_1 = __importDefault(require("node:stream"));
const node_tty_1 = __importDefault(require("node:tty"));
const readSync_js_1 = require("../functions/readSync.js");
const writeSync_js_1 = require("../functions/writeSync.js");
function chain(fn, cleanup) {
    let prev;
    return function _fn(...args) {
        const curr = (prev || Promise.resolve())
            .then(() => fn(...args))
            .finally(cleanup || (() => { }))
            .then((result) => {
            if (prev === curr)
                prev = undefined;
            return result;
        });
        return (prev = curr);
    };
}
let stdinReadable;
exports.stdin = {
    rid: 0,
    isTerminal() {
        return node_tty_1.default.isatty(this.rid);
    },
    read: chain((p) => {
        return new Promise((resolve, reject) => {
            process.stdin.resume();
            process.stdin.on("error", onerror);
            process.stdin.once("readable", () => {
                var _a;
                process.stdin.off("error", onerror);
                const data = (_a = process.stdin.read(p.length)) !== null && _a !== void 0 ? _a : process.stdin.read();
                if (data) {
                    p.set(data);
                    resolve(data.length > 0 ? data.length : null);
                }
                else {
                    resolve(null);
                }
            });
            function onerror(error) {
                reject(error);
                process.stdin.off("error", onerror);
            }
        });
    }, () => process.stdin.pause()),
    get readable() {
        if (stdinReadable == null) {
            stdinReadable = node_stream_1.default.Readable.toWeb(process.stdin);
        }
        return stdinReadable;
    },
    readSync(buffer) {
        return (0, readSync_js_1.readSync)(this.rid, buffer);
    },
    close() {
        process.stdin.destroy();
    },
    setRaw(mode, options) {
        if (options === null || options === void 0 ? void 0 : options.cbreak) {
            throw new Error("The cbreak option is not implemented.");
        }
        process.stdin.setRawMode(mode);
    },
};
let stdoutWritable;
exports.stdout = {
    rid: 1,
    isTerminal() {
        return node_tty_1.default.isatty(this.rid);
    },
    write: chain((p) => {
        return new Promise((resolve) => {
            const result = process.stdout.write(p);
            if (!result) {
                process.stdout.once("drain", () => resolve(p.length));
            }
            else {
                resolve(p.length);
            }
        });
    }),
    get writable() {
        if (stdoutWritable == null) {
            stdoutWritable = node_stream_1.default.Writable.toWeb(process.stdout);
        }
        return stdoutWritable;
    },
    writeSync(data) {
        return (0, writeSync_js_1.writeSync)(this.rid, data);
    },
    close() {
        process.stdout.destroy();
    },
};
let stderrWritable;
exports.stderr = {
    rid: 2,
    isTerminal() {
        return node_tty_1.default.isatty(this.rid);
    },
    write: chain((p) => {
        return new Promise((resolve) => {
            const result = process.stderr.write(p);
            if (!result) {
                process.stderr.once("drain", () => resolve(p.length));
            }
            else {
                resolve(p.length);
            }
        });
    }),
    get writable() {
        if (stderrWritable == null) {
            stderrWritable = node_stream_1.default.Writable.toWeb(process.stderr);
        }
        return stderrWritable;
    },
    writeSync(data) {
        return (0, writeSync_js_1.writeSync)(this.rid, data);
    },
    close() {
        process.stderr.destroy();
    },
};
