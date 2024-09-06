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
export declare function getFileInfoType(fileInfo: dntShim.Deno.FileInfo): PathType | undefined;
//# sourceMappingURL=_get_file_info_type.d.ts.map