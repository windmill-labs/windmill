// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { extname as posixExtname } from "./posix/extname.js";
import { extname as windowsExtname } from "./windows/extname.js";
export function extname(path) {
    return isWindows ? windowsExtname(path) : posixExtname(path);
}
