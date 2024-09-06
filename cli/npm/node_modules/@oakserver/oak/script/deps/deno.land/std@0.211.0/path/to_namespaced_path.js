"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.toNamespacedPath = void 0;
const _os_js_1 = require("./_os.js");
const to_namespaced_path_js_1 = require("./posix/to_namespaced_path.js");
const to_namespaced_path_js_2 = require("./windows/to_namespaced_path.js");
/**
 * Resolves path to a namespace path
 * @param path to resolve to namespace
 */
function toNamespacedPath(path) {
    return _os_js_1.isWindows
        ? (0, to_namespaced_path_js_2.toNamespacedPath)(path)
        : (0, to_namespaced_path_js_1.toNamespacedPath)(path);
}
exports.toNamespacedPath = toNamespacedPath;
