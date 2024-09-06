"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.dirname = void 0;
const _os_js_1 = require("./_os.js");
const dirname_js_1 = require("./posix/dirname.js");
const dirname_js_2 = require("./windows/dirname.js");
/**
 * Return the directory path of a `path`.
 * @param path - path to extract the directory from.
 */
function dirname(path) {
    return _os_js_1.isWindows ? (0, dirname_js_2.dirname)(path) : (0, dirname_js_1.dirname)(path);
}
exports.dirname = dirname;
