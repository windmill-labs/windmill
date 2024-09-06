// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright the Browserify authors. MIT License.
// Ported from https://github.com/browserify/path-browserify/
// This module is browser compatible.
import { CHAR_BACKWARD_SLASH, CHAR_FORWARD_SLASH, CHAR_LOWERCASE_A, CHAR_LOWERCASE_Z, CHAR_UPPERCASE_A, CHAR_UPPERCASE_Z, } from "../_common/constants.js";
export function isPosixPathSeparator(code) {
    return code === CHAR_FORWARD_SLASH;
}
export function isPathSeparator(code) {
    return code === CHAR_FORWARD_SLASH || code === CHAR_BACKWARD_SLASH;
}
export function isWindowsDeviceRoot(code) {
    return ((code >= CHAR_LOWERCASE_A && code <= CHAR_LOWERCASE_Z) ||
        (code >= CHAR_UPPERCASE_A && code <= CHAR_UPPERCASE_Z));
}
