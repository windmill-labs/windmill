"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright the Browserify authors. MIT License.
// Ported from https://github.com/browserify/path-browserify/
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.isPosixPathSeparator = void 0;
const constants_js_1 = require("../_common/constants.js");
function isPosixPathSeparator(code) {
    return code === constants_js_1.CHAR_FORWARD_SLASH;
}
exports.isPosixPathSeparator = isPosixPathSeparator;
