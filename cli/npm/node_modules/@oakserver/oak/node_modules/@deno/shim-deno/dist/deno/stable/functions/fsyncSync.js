"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.fsyncSync = void 0;
const fs_1 = require("fs");
const fsyncSync = function fsyncSync(rid) {
    return (0, fs_1.fsyncSync)(rid);
};
exports.fsyncSync = fsyncSync;
