var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var _FileHandler_instances, _FileHandler_unloadCallback, _FileHandler_resetBuffer, _a, _b, _c, _d, _e, _f, _g;
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import * as dntShim from "../../../../../_dnt.shims.js";
import { LogLevels } from "./levels.js";
import { BaseHandler } from "./base_handler.js";
import { writeAllSync } from "../../io/0.224.7/write_all.js";
import { bufSymbol, encoderSymbol, filenameSymbol, fileSymbol, modeSymbol, openOptionsSymbol, pointerSymbol, } from "./_file_handler_symbols.js";
/**
 * This handler will output to a file using an optional mode (default is `a`,
 * e.g. append). The file will grow indefinitely. It uses a buffer for writing
 * to file. Logs can be manually flushed with `fileHandler.flush()`. Log
 * messages with a log level greater than error are immediately flushed. Logs
 * are also flushed on process completion.
 *
 * Behavior of the log modes is as follows:
 *
 * - `'a'` - Default mode. Appends new log messages to the end of an existing log
 *   file, or create a new log file if none exists.
 * - `'w'` - Upon creation of the handler, any existing log file will be removed
 *   and a new one created.
 * - `'x'` - This will create a new log file and throw an error if one already
 *   exists.
 *
 * This handler requires `--allow-write` permission on the log file.
 */
export class FileHandler extends BaseHandler {
    constructor(levelName, options) {
        super(levelName, options);
        _FileHandler_instances.add(this);
        Object.defineProperty(this, _a, {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, _b, {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, _c, {
            enumerable: true,
            configurable: true,
            writable: true,
            value: 0
        });
        Object.defineProperty(this, _d, {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, _e, {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, _f, {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, _g, {
            enumerable: true,
            configurable: true,
            writable: true,
            value: new TextEncoder()
        });
        _FileHandler_unloadCallback.set(this, (() => {
            this.destroy();
        }).bind(this));
        this[filenameSymbol] = options.filename;
        // default to append mode, write only
        this[modeSymbol] = options.mode ?? "a";
        this[openOptionsSymbol] = {
            createNew: this[modeSymbol] === "x",
            create: this[modeSymbol] !== "x",
            append: this[modeSymbol] === "a",
            truncate: this[modeSymbol] !== "a",
            write: true,
        };
        this[bufSymbol] = new Uint8Array(options.bufferSize ?? 4096);
    }
    setup() {
        this[fileSymbol] = dntShim.Deno.openSync(this[filenameSymbol], this[openOptionsSymbol]);
        __classPrivateFieldGet(this, _FileHandler_instances, "m", _FileHandler_resetBuffer).call(this);
        addEventListener("unload", __classPrivateFieldGet(this, _FileHandler_unloadCallback, "f"));
    }
    handle(logRecord) {
        super.handle(logRecord);
        // Immediately flush if log level is higher than ERROR
        if (logRecord.level > LogLevels.ERROR) {
            this.flush();
        }
    }
    log(msg) {
        const bytes = this[encoderSymbol].encode(msg + "\n");
        if (bytes.byteLength > this[bufSymbol].byteLength - this[pointerSymbol]) {
            this.flush();
        }
        if (bytes.byteLength > this[bufSymbol].byteLength) {
            writeAllSync(this[fileSymbol], bytes);
        }
        else {
            this[bufSymbol].set(bytes, this[pointerSymbol]);
            this[pointerSymbol] += bytes.byteLength;
        }
    }
    flush() {
        if (this[pointerSymbol] > 0 && this[fileSymbol]) {
            let written = 0;
            while (written < this[pointerSymbol]) {
                written += this[fileSymbol].writeSync(this[bufSymbol].subarray(written, this[pointerSymbol]));
            }
            __classPrivateFieldGet(this, _FileHandler_instances, "m", _FileHandler_resetBuffer).call(this);
        }
    }
    destroy() {
        this.flush();
        this[fileSymbol]?.close();
        this[fileSymbol] = undefined;
        removeEventListener("unload", __classPrivateFieldGet(this, _FileHandler_unloadCallback, "f"));
    }
}
_FileHandler_unloadCallback = new WeakMap(), _FileHandler_instances = new WeakSet(), _a = fileSymbol, _b = bufSymbol, _c = pointerSymbol, _d = filenameSymbol, _e = modeSymbol, _f = openOptionsSymbol, _g = encoderSymbol, _FileHandler_resetBuffer = function _FileHandler_resetBuffer() {
    this[pointerSymbol] = 0;
};
