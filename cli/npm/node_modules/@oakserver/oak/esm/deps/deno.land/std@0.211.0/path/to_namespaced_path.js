// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { toNamespacedPath as posixToNamespacedPath } from "./posix/to_namespaced_path.js";
import { toNamespacedPath as windowsToNamespacedPath } from "./windows/to_namespaced_path.js";
/**
 * Resolves path to a namespace path
 * @param path to resolve to namespace
 */
export function toNamespacedPath(path) {
    return isWindows
        ? windowsToNamespacedPath(path)
        : posixToNamespacedPath(path);
}
