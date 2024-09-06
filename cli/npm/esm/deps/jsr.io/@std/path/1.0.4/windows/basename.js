// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { assertArgs, lastPathSegment, stripSuffix, } from "../_common/basename.js";
import { CHAR_COLON } from "../_common/constants.js";
import { stripTrailingSeparators } from "../_common/strip_trailing_separators.js";
import { isPathSeparator, isWindowsDeviceRoot } from "./_util.js";
import { fromFileUrl } from "./from_file_url.js";
export function basename(path, suffix = "") {
    path = path instanceof URL ? fromFileUrl(path) : path;
    assertArgs(path, suffix);
    // Check for a drive letter prefix so as not to mistake the following
    // path separator as an extra separator at the end of the path that can be
    // disregarded
    let start = 0;
    if (path.length >= 2) {
        const drive = path.charCodeAt(0);
        if (isWindowsDeviceRoot(drive)) {
            if (path.charCodeAt(1) === CHAR_COLON)
                start = 2;
        }
    }
    const lastSegment = lastPathSegment(path, isPathSeparator, start);
    const strippedSegment = stripTrailingSeparators(lastSegment, isPathSeparator);
    return suffix ? stripSuffix(strippedSegment, suffix) : strippedSegment;
}
