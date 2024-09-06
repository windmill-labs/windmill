"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.resolve = void 0;
const _os_js_1 = require("./_os.js");
const resolve_js_1 = require("./posix/resolve.js");
const resolve_js_2 = require("./windows/resolve.js");
/**
 * Resolves path segments into a `path`
 * @param pathSegments to process to path
 */
function resolve(...pathSegments) {
    return _os_js_1.isWindows
        ? (0, resolve_js_2.resolve)(...pathSegments)
        : (0, resolve_js_1.resolve)(...pathSegments);
}
exports.resolve = resolve;
