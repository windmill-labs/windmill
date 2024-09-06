// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright the Browserify authors. MIT License.
// Ported mostly from https://github.com/browserify/path-browserify/
// This module is browser compatible.
/**
 * Utilities for working with OS-specific file paths.
 *
 * Functions from this module will automatically switch to support the path style
 * of the current OS, either `windows` for Microsoft Windows, or `posix` for
 * every other operating system, eg. Linux, MacOS, BSD etc.
 *
 * To use functions for a specific path style regardless of the current OS
 * import the modules from the platform sub directory instead.
 *
 * Example, for POSIX:
 *
 * ```ts
 * import { fromFileUrl } from "@std/path/posix/from-file-url";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(fromFileUrl("file:///home/foo"), "/home/foo");
 * ```
 *
 * Or, for Windows:
 *
 * ```ts
 * import { fromFileUrl } from "@std/path/windows/from-file-url";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(fromFileUrl("file:///home/foo"), "\\home\\foo");
 * ```
 *
 * Functions for working with URLs can be found in
 * {@link ./doc/posix/~ | @std/path/posix}.
 *
 * @module
 */
export * from "./basename.js";
export * from "./constants.js";
export * from "./dirname.js";
export * from "./extname.js";
export * from "./format.js";
export * from "./from_file_url.js";
export * from "./is_absolute.js";
export * from "./join.js";
export * from "./normalize.js";
export * from "./parse.js";
export * from "./relative.js";
export * from "./resolve.js";
export * from "./to_file_url.js";
export * from "./to_namespaced_path.js";
export * from "./common.js";
export * from "./types.js";
export * from "./glob_to_regexp.js";
export * from "./is_glob.js";
export * from "./join_globs.js";
export * from "./normalize_glob.js";
