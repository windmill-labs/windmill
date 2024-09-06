// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { basename as posixBasename } from "./posix/basename.js";
import { basename as windowsBasename } from "./windows/basename.js";
export function basename(path, suffix = "") {
    return isWindows
        ? windowsBasename(path, suffix)
        : posixBasename(path, suffix);
}
