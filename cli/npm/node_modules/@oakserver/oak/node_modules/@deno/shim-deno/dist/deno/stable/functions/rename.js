"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.rename = void 0;
const promises_1 = require("fs/promises");
const rename = function rename(oldpath, newpath) {
    return (0, promises_1.rename)(oldpath, newpath);
};
exports.rename = rename;
