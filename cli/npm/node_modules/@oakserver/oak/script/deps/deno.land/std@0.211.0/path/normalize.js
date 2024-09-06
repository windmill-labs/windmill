"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.normalize = void 0;
const _os_js_1 = require("./_os.js");
const normalize_js_1 = require("./posix/normalize.js");
const normalize_js_2 = require("./windows/normalize.js");
/**
 * Normalize the `path`, resolving `'..'` and `'.'` segments.
 * Note that resolving these segments does not necessarily mean that all will be eliminated.
 * A `'..'` at the top-level will be preserved, and an empty path is canonically `'.'`.
 * @param path to be normalized
 */
function normalize(path) {
    return _os_js_1.isWindows ? (0, normalize_js_2.normalize)(path) : (0, normalize_js_1.normalize)(path);
}
exports.normalize = normalize;
