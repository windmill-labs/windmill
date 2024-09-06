// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { assertPath } from "./assert_path.js";
export function assertArgs(from, to) {
    assertPath(from);
    assertPath(to);
    if (from === to)
        return "";
}
