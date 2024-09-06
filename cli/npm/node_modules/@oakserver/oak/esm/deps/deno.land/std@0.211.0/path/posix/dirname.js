// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { assertArg } from "../_common/dirname.js";
import { stripTrailingSeparators } from "../_common/strip_trailing_separators.js";
import { isPosixPathSeparator } from "./_util.js";
/**
 * Return the directory path of a `path`.
 * @param path - path to extract the directory from.
 */
export function dirname(path) {
    assertArg(path);
    let end = -1;
    let matchedNonSeparator = false;
    for (let i = path.length - 1; i >= 1; --i) {
        if (isPosixPathSeparator(path.charCodeAt(i))) {
            if (matchedNonSeparator) {
                end = i;
                break;
            }
        }
        else {
            matchedNonSeparator = true;
        }
    }
    // No matches. Fallback based on provided path:
    //
    // - leading slashes paths
    //     "/foo" => "/"
    //     "///foo" => "/"
    // - no slash path
    //     "foo" => "."
    if (end === -1) {
        return isPosixPathSeparator(path.charCodeAt(0)) ? "/" : ".";
    }
    return stripTrailingSeparators(path.slice(0, end), isPosixPathSeparator);
}
