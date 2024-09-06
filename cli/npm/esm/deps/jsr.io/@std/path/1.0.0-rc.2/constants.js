// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { isWindows } from "./_os.js";
/**
 * The character used to separate entries in the PATH environment variable.
 * On Windows, this is `;`. On all other platforms, this is `:`.
 */
export const DELIMITER = isWindows ? ";" : ":";
/**
 * The character used to separate components of a file path.
 * On Windows, this is `\`. On all other platforms, this is `/`.
 */
export const SEPARATOR = isWindows ? "\\" : "/";
/**
 * A regular expression that matches one or more path separators.
 */
export const SEPARATOR_PATTERN = isWindows ? /[\\/]+/ : /\/+/;
