"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.makeTempFileSync = void 0;
const os_1 = require("os");
const path_1 = require("path");
const random_id_js_1 = require("../../internal/random_id.js");
const writeTextFileSync_js_1 = require("./writeTextFileSync.js");
const makeTempFileSync = function makeTempFileSync({ prefix = "" } = {}) {
    const name = (0, path_1.join)((0, os_1.tmpdir)(), prefix, (0, random_id_js_1.randomId)());
    (0, writeTextFileSync_js_1.writeTextFileSync)(name, "");
    return name;
};
exports.makeTempFileSync = makeTempFileSync;
