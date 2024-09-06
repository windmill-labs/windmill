"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.makeTempFile = void 0;
const os_1 = require("os");
const path_1 = require("path");
const random_id_js_1 = require("../../internal/random_id.js");
const writeTextFile_js_1 = require("./writeTextFile.js");
const makeTempFile = async function makeTempFile({ prefix = "" } = {}) {
    const name = (0, path_1.join)((0, os_1.tmpdir)(), prefix, (0, random_id_js_1.randomId)());
    await (0, writeTextFile_js_1.writeTextFile)(name, "");
    return name;
};
exports.makeTempFile = makeTempFile;
