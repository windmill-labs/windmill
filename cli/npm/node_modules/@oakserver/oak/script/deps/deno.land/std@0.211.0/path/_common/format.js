"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.assertArg = exports._format = void 0;
function _format(sep, pathObject) {
    const dir = pathObject.dir || pathObject.root;
    const base = pathObject.base ||
        (pathObject.name || "") + (pathObject.ext || "");
    if (!dir)
        return base;
    if (base === sep)
        return dir;
    if (dir === pathObject.root)
        return dir + base;
    return dir + sep + base;
}
exports._format = _format;
function assertArg(pathObject) {
    if (pathObject === null || typeof pathObject !== "object") {
        throw new TypeError(`The "pathObject" argument must be of type Object. Received type ${typeof pathObject}`);
    }
}
exports.assertArg = assertArg;
