// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { assertPath } from "./assert_path.js";
export function assertArg(path) {
    assertPath(path);
    if (path.length === 0)
        return ".";
}
