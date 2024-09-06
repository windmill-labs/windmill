"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.createSync = void 0;
const openSync_js_1 = require("./openSync.js");
const createSync = function createSync(path) {
    return (0, openSync_js_1.openSync)(path, {
        create: true,
        truncate: true,
        read: true,
        write: true,
    });
};
exports.createSync = createSync;
