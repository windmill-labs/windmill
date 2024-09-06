/**
 * Get a human readable file type string.
 *
 * @param file File information, as returned by {@linkcode Deno.stat} or
 * {@linkcode Deno.lstat}.
 *
 * @returns The file type as a string, or `undefined` if the file type is
 * unknown.
 */
export function getFileInfoType(fileInfo) {
    return fileInfo.isFile
        ? "file"
        : fileInfo.isDirectory
            ? "dir"
            : fileInfo.isSymlink
                ? "symlink"
                : undefined;
}
