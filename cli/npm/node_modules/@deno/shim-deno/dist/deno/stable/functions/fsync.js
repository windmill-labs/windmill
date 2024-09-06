"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.fsync = void 0;
const fs_1 = require("fs");
const util_1 = require("util");
const fsync = function fsync(rid) {
    return (0, util_1.promisify)(fs_1.fsync)(rid);
};
exports.fsync = fsync;
