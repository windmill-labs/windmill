"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.join = void 0;
const _os_js_1 = require("./_os.js");
const join_js_1 = require("./posix/join.js");
const join_js_2 = require("./windows/join.js");
/**
 * Join all given a sequence of `paths`,then normalizes the resulting path.
 * @param paths to be joined and normalized
 */
function join(...paths) {
    return _os_js_1.isWindows ? (0, join_js_2.join)(...paths) : (0, join_js_1.join)(...paths);
}
exports.join = join;
