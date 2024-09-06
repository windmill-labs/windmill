"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.copy = void 0;
const consts_js_1 = require("../../internal/consts.js");
const copy = async function copy(src, dst, options) {
    var _a;
    let n = 0;
    const bufSize = (_a = options === null || options === void 0 ? void 0 : options.bufSize) !== null && _a !== void 0 ? _a : consts_js_1.DEFAULT_BUFFER_SIZE;
    const b = new Uint8Array(bufSize);
    let gotEOF = false;
    while (gotEOF === false) {
        const result = await src.read(b);
        if (result === null) {
            gotEOF = true;
        }
        else {
            let nwritten = 0;
            while (nwritten < result) {
                nwritten += await dst.write(b.subarray(nwritten, result));
            }
            n += nwritten;
        }
    }
    return n;
};
exports.copy = copy;
