"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.normalizeGlob = void 0;
const _os_js_1 = require("./_os.js");
const normalize_glob_js_1 = require("./posix/normalize_glob.js");
const normalize_glob_js_2 = require("./windows/normalize_glob.js");
/** Like normalize(), but doesn't collapse "**\/.." when `globstar` is true. */
function normalizeGlob(glob, options = {}) {
    return _os_js_1.isWindows
        ? (0, normalize_glob_js_2.normalizeGlob)(glob, options)
        : (0, normalize_glob_js_1.normalizeGlob)(glob, options);
}
exports.normalizeGlob = normalizeGlob;
