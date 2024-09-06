// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright the Browserify authors. MIT License.
import * as dntShim from "../../../../../_dnt.shims.js";


export type PathType = "file" | "dir" | "symlink";

/**
 * Get a human readable file type string.
 *
 * @param file File information, as returned by {@linkcode Deno.stat} or
 * {@linkcode Deno.lstat}.
 *
 * @returns The file type as a string, or `undefined` if the file type is
 * unknown.
 */
export function getFileInfoType(fileInfo: dntShim.Deno.FileInfo): PathType | undefined {
  return fileInfo.isFile
    ? "file"
    : fileInfo.isDirectory
    ? "dir"
    : fileInfo.isSymlink
    ? "symlink"
    : undefined;
}
