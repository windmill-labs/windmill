// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { join as posixJoin } from "./posix/join.js";
import { join as windowsJoin } from "./windows/join.js";
export function join(path, ...paths) {
    return isWindows ? windowsJoin(path, ...paths) : posixJoin(path, ...paths);
}
