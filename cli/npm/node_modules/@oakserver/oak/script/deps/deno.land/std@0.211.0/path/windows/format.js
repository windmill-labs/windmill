"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.format = void 0;
const format_js_1 = require("../_common/format.js");
/**
 * Generate a path from `FormatInputPathObject` object.
 * @param pathObject with path
 */
function format(pathObject) {
    (0, format_js_1.assertArg)(pathObject);
    return (0, format_js_1._format)("\\", pathObject);
}
exports.format = format;
