"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.remove = void 0;
const promises_1 = require("fs/promises");
const remove = async function remove(path, options = {}) {
    const innerOptions = options.recursive
        ? { recursive: true, force: true }
        : {};
    try {
        return await (0, promises_1.rm)(path, innerOptions);
    }
    catch (err) {
        if (err.code === "ERR_FS_EISDIR") {
            return await (0, promises_1.rmdir)(path, innerOptions);
        }
        else {
            throw err;
        }
    }
};
exports.remove = remove;
