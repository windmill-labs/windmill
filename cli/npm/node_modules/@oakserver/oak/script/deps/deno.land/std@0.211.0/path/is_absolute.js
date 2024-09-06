"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.isAbsolute = void 0;
const _os_js_1 = require("./_os.js");
const is_absolute_js_1 = require("./posix/is_absolute.js");
const is_absolute_js_2 = require("./windows/is_absolute.js");
/**
 * Verifies whether provided path is absolute
 * @param path to be verified as absolute
 */
function isAbsolute(path) {
    return _os_js_1.isWindows ? (0, is_absolute_js_2.isAbsolute)(path) : (0, is_absolute_js_1.isAbsolute)(path);
}
exports.isAbsolute = isAbsolute;
