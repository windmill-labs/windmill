"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.read = void 0;
const util_1 = require("util");
const fs_1 = require("fs");
const _read = (0, util_1.promisify)(fs_1.read);
const read = async function read(rid, buffer) {
    if (buffer == null) {
        throw new TypeError("Buffer must not be null.");
    }
    if (buffer.length === 0) {
        return 0;
    }
    const { bytesRead } = await _read(rid, buffer, 0, buffer.length, null);
    // node returns 0 on EOF, Deno expects null
    return bytesRead === 0 ? null : bytesRead;
};
exports.read = read;
