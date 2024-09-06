"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.normalize = void 0;
const normalize_js_1 = require("../_common/normalize.js");
const normalize_string_js_1 = require("../_common/normalize_string.js");
const _util_js_1 = require("./_util.js");
/**
 * Normalize the `path`, resolving `'..'` and `'.'` segments.
 * Note that resolving these segments does not necessarily mean that all will be eliminated.
 * A `'..'` at the top-level will be preserved, and an empty path is canonically `'.'`.
 * @param path to be normalized
 */
function normalize(path) {
    (0, normalize_js_1.assertArg)(path);
    const isAbsolute = (0, _util_js_1.isPosixPathSeparator)(path.charCodeAt(0));
    const trailingSeparator = (0, _util_js_1.isPosixPathSeparator)(path.charCodeAt(path.length - 1));
    // Normalize the path
    path = (0, normalize_string_js_1.normalizeString)(path, !isAbsolute, "/", _util_js_1.isPosixPathSeparator);
    if (path.length === 0 && !isAbsolute)
        path = ".";
    if (path.length > 0 && trailingSeparator)
        path += "/";
    if (isAbsolute)
        return `/${path}`;
    return path;
}
exports.normalize = normalize;
