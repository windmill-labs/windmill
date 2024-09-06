"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.joinGlobs = void 0;
const _os_js_1 = require("./_os.js");
const join_globs_js_1 = require("./posix/join_globs.js");
const join_globs_js_2 = require("./windows/join_globs.js");
/** Like join(), but doesn't collapse "**\/.." when `globstar` is true. */
function joinGlobs(globs, options = {}) {
    return _os_js_1.isWindows
        ? (0, join_globs_js_2.joinGlobs)(globs, options)
        : (0, join_globs_js_1.joinGlobs)(globs, options);
}
exports.joinGlobs = joinGlobs;
