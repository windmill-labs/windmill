"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright the Browserify authors. MIT License.
// Ported from https://github.com/browserify/path-browserify/
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.isWindowsDeviceRoot = exports.isPathSeparator = exports.isPosixPathSeparator = void 0;
const constants_js_1 = require("../_common/constants.js");
function isPosixPathSeparator(code) {
    return code === constants_js_1.CHAR_FORWARD_SLASH;
}
exports.isPosixPathSeparator = isPosixPathSeparator;
function isPathSeparator(code) {
    return code === constants_js_1.CHAR_FORWARD_SLASH || code === constants_js_1.CHAR_BACKWARD_SLASH;
}
exports.isPathSeparator = isPathSeparator;
function isWindowsDeviceRoot(code) {
    return ((code >= constants_js_1.CHAR_LOWERCASE_A && code <= constants_js_1.CHAR_LOWERCASE_Z) ||
        (code >= constants_js_1.CHAR_UPPERCASE_A && code <= constants_js_1.CHAR_UPPERCASE_Z));
}
exports.isWindowsDeviceRoot = isWindowsDeviceRoot;
