"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.format = void 0;
const _os_js_1 = require("./_os.js");
const format_js_1 = require("./posix/format.js");
const format_js_2 = require("./windows/format.js");
/**
 * Generate a path from `FormatInputPathObject` object.
 * @param pathObject with path
 */
function format(pathObject) {
    return _os_js_1.isWindows ? (0, format_js_2.format)(pathObject) : (0, format_js_1.format)(pathObject);
}
exports.format = format;
