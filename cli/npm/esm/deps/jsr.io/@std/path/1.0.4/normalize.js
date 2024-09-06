// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { normalize as posixNormalize } from "./posix/normalize.js";
import { normalize as windowsNormalize } from "./windows/normalize.js";
export function normalize(path) {
    return isWindows ? windowsNormalize(path) : posixNormalize(path);
}
