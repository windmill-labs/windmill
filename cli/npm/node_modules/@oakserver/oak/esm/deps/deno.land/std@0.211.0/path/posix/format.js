// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { _format, assertArg } from "../_common/format.js";
/**
 * Generate a path from `FormatInputPathObject` object.
 * @param pathObject with path
 */
export function format(pathObject) {
    assertArg(pathObject);
    return _format("/", pathObject);
}
