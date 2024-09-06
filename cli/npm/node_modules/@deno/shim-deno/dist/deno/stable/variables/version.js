"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.version = void 0;
const version_js_1 = require("../../internal/version.js");
exports.version = {
    deno: version_js_1.deno,
    typescript: version_js_1.typescript,
    v8: process.versions.v8,
};
