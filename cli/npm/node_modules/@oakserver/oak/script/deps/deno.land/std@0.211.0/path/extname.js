"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.extname = void 0;
const _os_js_1 = require("./_os.js");
const extname_js_1 = require("./posix/extname.js");
const extname_js_2 = require("./windows/extname.js");
/**
 * Return the extension of the `path` with leading period.
 * @param path with extension
 * @returns extension (ex. for `file.ts` returns `.ts`)
 */
function extname(path) {
    return _os_js_1.isWindows ? (0, extname_js_2.extname)(path) : (0, extname_js_1.extname)(path);
}
exports.extname = extname;
