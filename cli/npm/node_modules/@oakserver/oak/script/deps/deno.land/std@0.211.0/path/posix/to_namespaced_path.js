"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.toNamespacedPath = void 0;
/**
 * Resolves path to a namespace path
 * @param path to resolve to namespace
 */
function toNamespacedPath(path) {
    // Non-op on posix systems
    return path;
}
exports.toNamespacedPath = toNamespacedPath;
