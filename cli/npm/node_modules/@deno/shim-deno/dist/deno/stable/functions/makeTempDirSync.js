"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.makeTempDirSync = void 0;
const fs_1 = require("fs");
const path_1 = require("path");
const os_1 = require("os");
const makeTempDirSync = function makeTempDirSync({ prefix = "" } = {}) {
    return (0, fs_1.mkdtempSync)((0, path_1.join)((0, os_1.tmpdir)(), prefix || "/"));
};
exports.makeTempDirSync = makeTempDirSync;
