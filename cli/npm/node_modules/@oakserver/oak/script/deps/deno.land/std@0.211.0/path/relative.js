"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.relative = void 0;
const _os_js_1 = require("./_os.js");
const relative_js_1 = require("./posix/relative.js");
const relative_js_2 = require("./windows/relative.js");
/**
 * Return the relative path from `from` to `to` based on current working directory.
 *
 * An example in windws, for instance:
 *  from = 'C:\\orandea\\test\\aaa'
 *  to = 'C:\\orandea\\impl\\bbb'
 * The output of the function should be: '..\\..\\impl\\bbb'
 *
 * @param from path in current working directory
 * @param to path in current working directory
 */
function relative(from, to) {
    return _os_js_1.isWindows ? (0, relative_js_2.relative)(from, to) : (0, relative_js_1.relative)(from, to);
}
exports.relative = relative;
