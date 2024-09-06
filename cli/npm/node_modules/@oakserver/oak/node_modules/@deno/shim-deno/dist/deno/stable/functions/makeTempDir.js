"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.makeTempDir = void 0;
const promises_1 = require("fs/promises");
const path_1 = require("path");
const os_1 = require("os");
const makeTempDir = function makeTempDir({ prefix = "" } = {}) {
    return (0, promises_1.mkdtemp)((0, path_1.join)((0, os_1.tmpdir)(), prefix || "/"));
};
exports.makeTempDir = makeTempDir;
