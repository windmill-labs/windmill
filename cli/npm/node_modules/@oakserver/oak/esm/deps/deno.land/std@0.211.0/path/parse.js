// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
import { parse as posixParse } from "./posix/parse.js";
import { parse as windowsParse } from "./windows/parse.js";
/**
 * Return a `ParsedPath` object of the `path`.
 * @param path to process
 */
export function parse(path) {
    return isWindows ? windowsParse(path) : posixParse(path);
}
