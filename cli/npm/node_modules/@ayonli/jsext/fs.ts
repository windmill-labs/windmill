/**
 * Universal file system APIs for both server and browser applications.
 * 
 * This module is guaranteed to work in the following environments:
 * 
 * - Node.js
 * - Deno
 * - Bun
 * - Modern browsers
 * - Cloudflare Workers (limited support and experimental)
 * 
 * We can also use the {@link runtime} function to check whether the runtime
 * has file system support. When `runtime().fsSupport` is `true`, this module
 * should work properly.
 * 
 * In most browsers, this module uses the
 * [Origin Private File System](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system).
 * In Chromium browsers, this module can also access the device's local file
 * system via `window.showOpenFilePicker()` and `window.showDirectoryPicker()`.
 * 
 * This module also provides limited support for Cloudflare Workers, however it
 * requires setting the `[site].bucket` option in the `wrangler.toml` file. Only
 * the reading functions are supported, such as {@link readFile} and
 * {@link readDir}, these functions allow us reading static files in the workers,
 * writing functions is not implemented at the moment. More details about
 * serving static assets in Cloudflare Workers can be found here:
 * [Add static assets to an existing Workers project](https://developers.cloudflare.com/workers/configuration/sites/start-from-worker/).
 * 
 * **Errors:**
 * 
 * When a file system operation fails, this module throws an {@link Exception}
 * with one of the following names:
 * 
 * - `NotFoundError`: The file or directory does not exist.
 * - `NotAllowedError`: The operation is not allowed, such as being blocked by
 *   the permission system.
 * - `AlreadyExistsError`: The file or directory already exists.
 * - `IsDirectoryError`: The path is a directory, not a file.
 * - `NotDirectoryError`: The path is a file, not a directory.
 * - `InvalidOperationError`: The operation is not supported, such as trying to
 *   copy a directory without the `recursive` option.
 * - `BusyError`: The file is busy, such as being locked by another program.
 * - `InterruptedError`: The operation is interrupted by the underlying file
 *   system.
 * - `FileTooLargeError`: The file is too large, or the file system doesn't have
 *   enough space to store the new content.
 * - `FilesystemLoopError`:  Too many symbolic links were encountered when
 *   resolving the filename.
 * 
 * Other errors may also be thrown by the runtime, such as `TypeError`.
 * 
 * @experimental
 * @module
 */

import { abortable } from "./async.ts";
import bytes, { text } from "./bytes.ts";
import { isDeno, isNodeLike } from "./env.ts";
import { Exception } from "./error.ts";
import { getMIME } from "./filetype.ts";
import type { FileInfo, DirEntry, FileSystemOptions, DirTree } from "./fs/types.ts";
import { fixDirEntry, fixFileType, makeTree } from "./fs/util.ts";
import { as } from "./object.ts";
import { basename, dirname, extname, join, split } from "./path.ts";
import { readAsArray, resolveByteStream, toAsyncIterable } from "./reader.ts";
import runtime, { platform } from "./runtime.ts";
import { stripStart } from "./string.ts";
import _try from "./try.ts";

export type { FileSystemOptions, FileInfo, DirEntry, DirTree };

/**
 * @deprecated Use {@link FileSystemOptions} instead.
 */
export type CommonOptions = FileSystemOptions;

/**
 * Platform-specific end-of-line marker. The value is `\r\n` in Windows
 * server-side environments, and `\n` elsewhere.
 */
export const EOL: "\n" | "\r\n" = (() => {
    if (isDeno) {
        return Deno.build.os === "windows" ? "\r\n" : "\n";
    } else if (typeof process === "object" && typeof process.platform === "string") {
        return process.platform === "win32" ? "\r\n" : "\n";
    } else {
        return "\n";
    }
})();

function getErrorName(err: Error): string {
    if (err.constructor === Error) {
        return err.constructor.name;
    } else {
        return err.name;
    }
}

/**
 * Wraps a raw file system error to a predefined error by this module.
 * 
 * @param type Used for `FileSystemHandle` operations.
 */
function wrapFsError(
    err: unknown,
    type: "file" | "directory" | undefined = undefined
): Exception | Error {
    if (err instanceof Error && !(err instanceof Exception) && !(err instanceof TypeError)) {
        const errName = getErrorName(err);
        const errCode = (err as NodeJS.ErrnoException).code;

        if (errName === "NotFoundError"
            || errName === "NotFound"
            || errCode === "ENOENT"
            || errCode === "ENOTFOUND"
        ) {
            return new Exception(err.message, { name: "NotFoundError", code: 404, cause: err });
        } else if (errName === "NotAllowedError"
            || errName === "PermissionDenied"
            || errName === "InvalidStateError"
            || errName === "SecurityError"
            || errName === "EACCES"
            || errCode === "EPERM"
            || errCode === "ERR_ACCESS_DENIED"
        ) {
            return new Exception(err.message, { name: "NotAllowedError", code: 403, cause: err });
        } else if (errName === "AlreadyExists"
            || errCode === "EEXIST"
            || errCode === "ERR_FS_CP_EEXIST"
        ) {
            return new Exception(err.message, { name: "AlreadyExistsError", code: 409, cause: err });
        } else if ((errName === "TypeMismatchError" && type === "file")
            || errName === "IsADirectory"
            || errCode === "EISDIR"
            || errCode === "ERR_FS_EISDIR"
        ) {
            return new Exception(err.message, { name: "IsDirectoryError", code: 415, cause: err });
        } else if ((errName === "TypeMismatchError" && type === "directory")
            || errName === "NotADirectory"
            || errCode === "ENOTDIR"
        ) {
            return new Exception(err.message, { name: "NotDirectoryError", code: 415, cause: err });
        } else if (errName === "InvalidModificationError"
            || errName === "NotSupported"
            || errCode === "ENOTEMPTY"
            || errCode === "ERR_FS_CP_EINVAL"
            || errCode === "ERR_FS_CP_FIFO_PIPE"
            || errCode === "ERR_FS_CP_DIR_TO_NON_DIR"
            || errCode === "ERR_FS_CP_NON_DIR_TO_DIR"
            || errCode === "ERR_FS_CP_SOCKET"
            || errCode === "ERR_FS_CP_SYMLINK_TO_SUBDIRECTORY"
            || errCode === "ERR_FS_CP_UNKNOWN"
            || errCode === "ERR_FS_INVALID_SYMLINK_TYPE"
        ) {
            return new Exception(err.message, { name: "InvalidOperationError", code: 405, cause: err });
        } else if (errName === "NoModificationAllowedError"
            || errName === "Busy"
            || errName === "TimedOut"
            || errCode === "ERR_DIR_CONCURRENT_OPERATION"
        ) {
            return new Exception(errName, { name: "BusyError", code: 409, cause: err });
        } else if (errName === "Interrupted" || errCode === "ERR_DIR_CLOSED") {
            return new Exception(err.message, { name: "InterruptedError", code: 409, cause: err });
        } else if (errName === "QuotaExceededError"
            || errCode === "ERR_FS_FILE_TOO_LARGE"
        ) {
            return new Exception(err.message, { name: "FileTooLargeError", code: 413, cause: err });
        } else if (errName === "FilesystemLoop") {
            return new Exception(err.message, { name: "FilesystemLoopError", code: 508, cause: err });
        } else {
            return err;
        }
    } else if (err instanceof Error) {
        return err;
    } else if (typeof err === "string") {
        return new Exception(err, { code: 500, cause: err });
    } else {
        return new Exception("Unknown error", { code: 500, cause: err });
    }
}

/**
 * Wraps a raw file system operation so that when any error occurs, the error is
 * wrapped to a predefined error by this module.
 * 
 * @param type Only used for `FileSystemHandle` operations.
 */
function rawOp<T>(op: Promise<T>, type: "file" | "directory" | undefined = undefined): Promise<T> {
    return op.catch((err) => {
        throw wrapFsError(err, type);
    });
}

/**
 * Options for the {@link getDirHandle} function.
 */
export interface GetDirOptions extends FileSystemOptions {
    /**
     * Create the directory if not exist.
     */
    create?: boolean;
    /**
     * Used when `create` is `true`, recursively create the directory and its
     * parent directories.
     */
    recursive?: boolean;
}

/**
 * Obtains the directory handle of the given path.
 * 
 * NOTE: This function is only available in the browser.
 * 
 * NOTE: If the `path` is not provided or is empty, the root directory handle
 * will be returned.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { getDirHandle } from "@ayonli/jsext/fs";
 * 
 * const dir = await getDirHandle("/path/to/dir");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { getDirHandle } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * const dir = await getDirHandle("/path/to/dir", { root });
 * ```
 * 
 * @example
 * ```ts
 * // create the directory if not exist
 * import { getDirHandle } from "@ayonli/jsext/fs";
 * 
 * const dir = await getDirHandle("/path/to/dir", { create: true, recursive: true });
 * ```
 * 
 * @example
 * ```ts
 * // return the root directory handle
 * import { getDirHandle } from "@ayonli/jsext/fs";
 * 
 * const root = await getDirHandle();
 * ```
 */
export async function getDirHandle(
    path: string = "",
    options: GetDirOptions = {}
): Promise<FileSystemDirectoryHandle> {
    if (typeof location === "object" && typeof location.origin === "string") {
        path = stripStart(path, location.origin);
    }

    const { create = false, recursive = false } = options;
    const paths = split(stripStart(path, "/")).filter(p => p !== ".");
    const root = options.root ?? await rawOp(navigator.storage.getDirectory(), "directory");
    let dir = root as FileSystemDirectoryHandle;

    for (let i = 0; i < paths.length; i++) {
        const _path = paths[i]!;
        dir = await rawOp(dir.getDirectoryHandle(_path, {
            create: create && (recursive || (i === paths.length - 1)),
        }), "directory");
    }

    return dir;
}

/**
 * Options for the {@link getFileHandle} function.
 */
export interface GetFileOptions extends FileSystemOptions {
    /**
     * Create the file if not exist.
     */
    create?: boolean;
}

/**
 * Obtains the file handle of the given path.
 * 
 * NOTE: This function is only available in the browser.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { getFileHandle } from "@ayonli/jsext/fs";
 * 
 * const file = await getFileHandle("/path/to/file.txt");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { getFileHandle } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * const file = await getFileHandle("/path/to/file.txt", { root });
 * ```
 * 
 * @example
 * ```ts
 * // create the file if not exist
 * import { getFileHandle } from "@ayonli/jsext/fs";
 * 
 * const file = await getFileHandle("/path/to/file.txt", { create: true });
 * ```
 */
export async function getFileHandle(
    path: string,
    options: GetFileOptions = {}
): Promise<FileSystemFileHandle> {
    const dirPath = dirname(path);
    const name = basename(path);
    const dir = await getDirHandle(dirPath, { root: options.root });
    return await rawOp(dir.getFileHandle(name, {
        create: options.create ?? false,
    }), "file");
}

/**
 * Checks if the given path exists.
 * 
 * This function may throw an error if the path is invalid or the operation is
 * not allowed.
 * 
 * NOTE: This function can also be used in Cloudflare Workers.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { exists } from "@ayonli/jsext/fs";
 * 
 * if (await exists("/path/to/file.txt")) {
 *     console.log("The file exists.");
 * } else {
 *     console.log("The file does not exist.");
 * }
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { exists } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * 
 * if (await exists("/path/to/file.txt", { root })) {
 *     console.log("The file exists.");
 * } else {
 *     console.log("The file does not exist.");
 * }
 * ```
 */
export async function exists(path: string, options: FileSystemOptions = {}): Promise<boolean> {
    try {
        await stat(path, options);
        return true;
    } catch (err) {
        if (err instanceof Exception) {
            if (err.name === "NotFoundError") {
                return false;
            }
        }

        throw err;
    }
}

/**
 * Options for the {@link stat} function.
 */
export interface StatOptions extends FileSystemOptions {
    /**
     * Whether to follow the symbolic link.
     * @default false
     */
    followSymlink?: boolean;
}

/**
 * Returns the information of the given file or directory.
 * 
 * NOTE: This function can also be used in Cloudflare Workers.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { stat } from "@ayonli/jsext/fs";
 * 
 * const info = await stat("/path/to/file.txt");
 * console.log(`${info.name} is a ${info.kind}, its size is ${info.size} bytes, with MIME type ${info.type}.`);
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { stat } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * const info = await stat("/path/to/file.txt", { root });
 * console.log(`${info.name} is a ${info.kind}, its size is ${info.size} bytes, with MIME type ${info.type}.`);
 * ```
 */
export async function stat(
    target: string | FileSystemFileHandle | FileSystemDirectoryHandle,
    options: StatOptions = {}
): Promise<FileInfo> {
    if (typeof target === "object") {
        if (target.kind === "file") {
            const info = await rawOp(target.getFile(), "file");
            return {
                name: target.name,
                kind: "file",
                size: info.size,
                type: info.type ?? getMIME(extname(target.name)) ?? "",
                mtime: new Date(info.lastModified),
                atime: null,
                birthtime: null,
                mode: 0,
                uid: 0,
                gid: 0,
                isBlockDevice: false,
                isCharDevice: false,
                isFIFO: false,
                isSocket: false,
            };
        } else {
            return {
                name: target.name,
                kind: "directory",
                size: 0,
                type: "",
                mtime: null,
                atime: null,
                birthtime: null,
                mode: 0,
                uid: 0,
                gid: 0,
                isBlockDevice: false,
                isCharDevice: false,
                isFIFO: false,
                isSocket: false,
            };
        }
    }

    const path = target;

    if (isDeno) {
        const stat = await rawOp(options.followSymlink ? Deno.stat(path) : Deno.lstat(path));
        const kind = stat.isDirectory
            ? "directory"
            : stat.isSymlink
                ? "symlink"
                : "file";

        return {
            name: basename(path),
            kind,
            size: stat.size,
            type: kind === "file" ? (getMIME(extname(path)) ?? "") : "",
            mtime: stat.mtime ?? null,
            atime: stat.atime ?? null,
            birthtime: stat.birthtime ?? null,
            mode: stat.mode ?? 0,
            uid: stat.uid ?? 0,
            gid: stat.gid ?? 0,
            isBlockDevice: stat.isBlockDevice ?? false,
            isCharDevice: stat.isCharDevice ?? false,
            isFIFO: stat.isFifo ?? false,
            isSocket: stat.isSocket ?? false,
        };
    } else if (isNodeLike) {
        const fs = await import("fs/promises");
        const stat = await rawOp(options.followSymlink ? fs.stat(path) : fs.lstat(path));
        const kind = stat.isDirectory()
            ? "directory"
            : stat.isSymbolicLink()
                ? "symlink"
                : "file";

        return {
            name: basename(path),
            kind,
            size: stat.size,
            type: kind === "file" ? (getMIME(extname(path)) ?? "") : "",
            mtime: stat.mtime ?? null,
            atime: stat.atime ?? null,
            birthtime: stat.birthtime ?? null,
            mode: stat.mode ?? 0,
            uid: stat.uid ?? 0,
            gid: stat.gid ?? 0,
            isBlockDevice: stat.isBlockDevice(),
            isCharDevice: stat.isCharacterDevice(),
            isFIFO: stat.isFIFO(),
            isSocket: stat.isSocket(),
        };
    } else {
        const [err, file] = await _try(getFileHandle(path, options));

        if (file) {
            const info = await rawOp(file.getFile(), "file");
            return {
                name: info.name,
                kind: "file",
                size: info.size,
                type: info.type ?? getMIME(extname(info.name)) ?? "",
                mtime: new Date(info.lastModified),
                atime: null,
                birthtime: null,
                mode: 0,
                uid: 0,
                gid: 0,
                isBlockDevice: false,
                isCharDevice: false,
                isFIFO: false,
                isSocket: false,
            };
        } else if (as(err, Exception)?.name === "IsDirectoryError") {
            return {
                name: basename(path),
                kind: "directory",
                size: 0,
                type: "",
                mtime: null,
                atime: null,
                birthtime: null,
                mode: 0,
                uid: 0,
                gid: 0,
                isBlockDevice: false,
                isCharDevice: false,
                isFIFO: false,
                isSocket: false,
            };
        } else {
            throw err;
        }
    }
}

/**
 * Options for the {@link mkdir} function.
 */
export interface MkdirOptions extends FileSystemOptions {
    /**
     * Whether to create parent directories if they do not exist.
     */
    recursive?: boolean;
    /**
     * The permission mode of the directory.
     * 
     * NOTE: This option is ignored in the browser and in Windows.
     * @default 0o777
     */
    mode?: number;
}

/**
 * Creates a new directory with the given path.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { mkdir } from "@ayonli/jsext/fs";
 * 
 * await mkdir("/path/to/dir");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { mkdir } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * await mkdir("/path/to/dir", { root });
 * ```
 * 
 * @example
 * ```ts
 * // create the directory and its parent directories if not exist
 * import { mkdir } from "@ayonli/jsext/fs";
 * 
 * await mkdir("/path/to/dir", { recursive: true });
 * ```
 */
export async function mkdir(path: string, options: MkdirOptions = {}): Promise<void> {
    if (isDeno) {
        await rawOp(Deno.mkdir(path, options));
    } else if (isNodeLike) {
        const fs = await import("fs/promises");
        await rawOp(fs.mkdir(path, options));
    } else {
        if (await exists(path, { root: options.root })) {
            throw new Exception(`File or folder already exists, mkdir '${path}'`, {
                name: "AlreadyExistsError",
                code: 409,
            });
        }

        await getDirHandle(path, { ...options, create: true });
    }
}

/**
 * Ensures the directory exists, creating it (and any parent directory) if not.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { ensureDir } from "@ayonli/jsext/fs";
 * 
 * await ensureDir("/path/to/dir");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { ensureDir } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * await ensureDir("/path/to/dir", { root });
 * ```
 */
export async function ensureDir(
    path: string,
    options: Omit<MkdirOptions, "recursive"> = {}
): Promise<void> {
    if (await exists(path, options)) {
        return;
    }

    try {
        await mkdir(path, { ...options, recursive: true });
    } catch (err) {
        if (as(err, Exception)?.name === "AlreadyExistsError") {
            return;
        } else {
            throw err;
        }
    }
}

/**
 * Options for the {@link readDir} function.
 */
export interface ReadDirOptions extends FileSystemOptions {
    /**
     * Whether to read the sub-directories recursively.
     */
    recursive?: boolean;
}

/**
 * Reads the directory of the given path and iterates its entries.
 * 
 * NOTE: The order of the entries is not guaranteed.
 * 
 * NOTE: This function can also be used in Cloudflare Workers.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { readDir } from "@ayonli/jsext/fs";
 * 
 * for await (const entry of readDir("/path/to/dir")) {
 *     console.log(`${entry.name} is a ${entry.kind}, its relative path is '${entry.relativePath}'.`);
 * }
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { readDir } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * for await (const entry of readDir("/path/to/dir", { root })) {
 *     console.log(`${entry.name} is a ${entry.kind}, its relative path is '${entry.relativePath}'.`);
 * }
 * ```
 * 
 * @example
 * ```ts
 * // read the sub-directories recursively
 * import { readDir } from "@ayonli/jsext/fs";
 * 
 * for await (const entry of readDir("/path/to/dir", { recursive: true })) {
 *     console.log(`${entry.name} is a ${entry.kind}, its relative path is '${entry.relativePath}'.`);
 * }
 * ```
 */
export async function* readDir(
    target: string | FileSystemDirectoryHandle,
    options: ReadDirOptions = {}
): AsyncIterableIterator<DirEntry> {
    if (typeof target === "object") {
        yield* readDirHandle(target, options);
        return;
    }

    const path = target;

    if (isDeno) {
        yield* (async function* read(path: string, base: string): AsyncIterableIterator<DirEntry> {
            try {
                for await (const entry of Deno.readDir(path)) {
                    const _entry = fixDirEntry({
                        name: entry.name,
                        kind: entry.isDirectory
                            ? "directory"
                            : entry.isSymlink
                                ? "symlink"
                                : "file",
                        relativePath: join(base, entry.name),
                    });

                    yield _entry;

                    if (options?.recursive && entry.isDirectory) {
                        yield* read(join(path, entry.name), _entry.relativePath);
                    }
                }
            } catch (err) {
                throw wrapFsError(err);
            }
        })(path, "");
    } else if (isNodeLike) {
        const fs = await import("fs/promises");

        yield* (async function* read(path: string, base: string): AsyncIterableIterator<DirEntry> {
            const entries = await rawOp(fs.readdir(path, { withFileTypes: true }));

            for (const entry of entries) {
                const _entry = fixDirEntry({
                    name: entry.name,
                    kind: entry.isDirectory()
                        ? "directory"
                        : entry.isSymbolicLink()
                            ? "symlink"
                            : "file",
                    relativePath: join(base, entry.name),
                });

                yield _entry;

                if (options?.recursive && entry.isDirectory()) {
                    yield* read(join(path, entry.name), _entry.relativePath);
                }
            }
        })(path, "");
    } else {
        const dir = await getDirHandle(path, { root: options.root });
        yield* readDirHandle(dir, options);
    }
}

/**
 * Recursively reads the contents of the directory and transform them into a
 * tree structure.
 * 
 * NOTE: Unlike {@link readDir}, the order of the entries returned by this
 * function is guaranteed, they are ordered first by kind (directories before
 * files), then by names alphabetically.
 * 
 * NOTE: This function can also be used in Cloudflare Workers.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { readTree } from "@ayonli/jsext/fs";
 * 
 * const tree = await readTree("/path/to/dir");
 * console.log(tree);
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { readTree } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * const tree = await readTree("/path/to/dir", { root });
 * console.log(tree);
 * ```
 */
export async function readTree(
    target: string | FileSystemDirectoryHandle,
    options: FileSystemOptions = {}
): Promise<DirTree> {
    const entries = (await readAsArray(readDir(target, { ...options, recursive: true })));
    const tree = makeTree<DirEntry, DirTree>(target, entries, true);

    if (!tree.handle && options.root) {
        tree.handle = options.root as FileSystemDirectoryHandle;
    }

    return tree;
}

async function* readDirHandle(dir: FileSystemDirectoryHandle, options: {
    base?: string,
    recursive?: boolean;
} = {}): AsyncIterableIterator<DirEntry> {
    const { base = "", recursive = false } = options;
    const entries = dir.entries();

    for await (const [_, entry] of entries) {
        const _entry = fixDirEntry({
            name: entry.name,
            kind: entry.kind,
            relativePath: join(base, entry.name),
            handle: entry as FileSystemFileHandle | FileSystemDirectoryHandle,
        });

        yield _entry;

        if (recursive && entry.kind === "directory") {
            yield* readDirHandle(entry as FileSystemDirectoryHandle, {
                base: _entry.relativePath,
                recursive,
            });
        }
    }
}

async function readFileHandle(handle: FileSystemFileHandle, options: {
    signal?: AbortSignal | undefined;
}): Promise<Uint8Array> {
    const file = await rawOp(handle.getFile(), "file");
    const arr = new Uint8Array(file.size);
    let offset = 0;
    let reader = toAsyncIterable(file.stream());

    if (options.signal) {
        reader = abortable(reader, options.signal);
    }

    for await (const chunk of reader) {
        arr.set(chunk, offset);
        offset += chunk.length;
    }

    return arr;
}

/**
 * Options for file reading functions, such as {@link readFile},
 * {@link readFileAsText} and {@link readFileAsFile}.
 */
export interface ReadFileOptions extends FileSystemOptions {
    signal?: AbortSignal;
}

/**
 * Reads the content of the given file in bytes.
 * 
 * NOTE: This function can also be used in Cloudflare Workers.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { readFile } from "@ayonli/jsext/fs";
 * 
 * const bytes = await readFile("/path/to/file.txt");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { readFile } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * const bytes = await readFile("/path/to/file.txt", { root });
 * ```
 */
export async function readFile(
    target: string | FileSystemFileHandle,
    options: ReadFileOptions = {}
): Promise<Uint8Array> {
    if (typeof target === "object") {
        return await readFileHandle(target, options);
    }

    const filename = target;

    if (isDeno) {
        return await rawOp(Deno.readFile(filename, options));
    } else if (isNodeLike) {
        const fs = await import("fs/promises");
        const buffer = await rawOp(fs.readFile(filename, options));
        return new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength);
    } else {
        const handle = await getFileHandle(filename, { root: options.root });
        return await readFileHandle(handle, options);
    }
}

/**
 * Reads the content of the given file as text with `utf-8` encoding.
 * 
 * NOTE: This function can also be used in Cloudflare Workers.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { readFileAsText } from "@ayonli/jsext/fs";
 * 
 * const text = await readFileAsText("/path/to/file.txt");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { readFileAsText } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * const text = await readFileAsText("/path/to/file.txt", { root });
 * ```
 */
export async function readFileAsText(
    target: string | FileSystemFileHandle,
    options: ReadFileOptions = {}
): Promise<string> {
    if (typeof target === "object") {
        return text(await readFileHandle(target, options));
    }

    const filename = target;

    if (isDeno) {
        return await rawOp(Deno.readTextFile(filename, options));
    } else if (isNodeLike) {
        const fs = await import("fs/promises");
        return await rawOp(fs.readFile(filename, {
            encoding: "utf-8",
            signal: options.signal,
        }));
    } else {
        return text(await readFile(filename, options));
    }
}

/**
 * Reads the file as a `File` object.
 * 
 * NOTE: This function can also be used in Cloudflare Workers.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { readFileAsFile } from "@ayonli/jsext/fs";
 * 
 * const file = await readFileAsFile("/path/to/file.txt");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { readFileAsFile } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * const file = await readFileAsFile("/path/to/file.txt", { root });
 * ```
 */
export async function readFileAsFile(
    target: string | FileSystemFileHandle,
    options: ReadFileOptions = {}
): Promise<File> {
    if (typeof target === "object") {
        return await readFileHandleAsFile(target);
    }

    const filename = target;

    if (isDeno || isNodeLike) {
        const bytes = await readFile(filename, options);
        const type = getMIME(extname(filename)) ?? "";
        const file = new File([bytes], basename(filename), { type });

        Object.defineProperty(file, "webkitRelativePath", {
            configurable: true,
            enumerable: true,
            writable: false,
            value: "",
        });

        return file;
    } else {
        const handle = await getFileHandle(target, { root: options.root });
        return await readFileHandleAsFile(handle);
    }
}

async function readFileHandleAsFile(handle: FileSystemFileHandle): Promise<File> {
    const file = await rawOp(handle.getFile(), "file");
    return fixFileType(file);
}

/**
 * Options for file writing functions, such as {@link writeFile} and {@link writeLines}.
 */
export interface WriteFileOptions extends FileSystemOptions {
    /**
     * Append the data to the file instead of overwriting it.
     */
    append?: boolean;
    /**
     * Permissions always applied to file.
     * 
     * NOTE: This option is ignored in the browser.
     * @default 0o666
     */
    mode?: number;
    signal?: AbortSignal;
}

/**
 * Writes the given data to the file.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { writeFile } from "@ayonli/jsext/fs";
 * 
 * await writeFile("/path/to/file.txt", "Hello, world!");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { writeFile } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * await writeFile("/path/to/file.txt", "Hello, world!", { root });
 * ```
 * 
 * @example
 * ```ts
 * // append the data to the file
 * import { writeFile } from "@ayonli/jsext/fs";
 * 
 * await writeFile("/path/to/file.txt", "Hello, world!", { append: true });
 * ```
 * 
 * @example
 * ```ts
 * // write binary data to the file
 * import { writeFile } from "@ayonli/jsext/fs";
 * import bytes from "@ayonli/jsext/bytes";
 * 
 * const data = bytes("Hello, world!");
 * await writeFile("/path/to/file.txt", data)
 * ```
 * 
 * @example
 * ```ts
 * // write a blob to the file
 * import { writeFile } from "@ayonli/jsext/fs";
 * 
 * const blob = new Blob(["Hello, world!"], { type: "text/plain" });
 * await writeFile("/path/to/file.txt", blob);
 * ```
 * 
 * @example
 * ```ts
 * // write a readable stream to the file
 * import { writeFile } from "@ayonli/jsext/fs";
 * 
 * const res = await fetch("https://example.com/file.txt");
 * await writeFile("/path/to/file.txt", res.body!);
 * ```
 */
export async function writeFile(
    target: string | FileSystemFileHandle,
    data: string | ArrayBuffer | ArrayBufferView | ReadableStream<Uint8Array> | Blob,
    options: WriteFileOptions = {}
): Promise<void> {
    if (typeof target === "object") {
        return await writeFileHandle(target, data, options);
    }

    const filename = target;

    if (isDeno) {
        if (typeof data === "string") {
            return await rawOp(Deno.writeTextFile(filename, data, options));
        } else if (data instanceof Blob) {
            return await rawOp(Deno.writeFile(filename, data.stream(), options));
        } else if (data instanceof ArrayBuffer) {
            return await rawOp(Deno.writeFile(filename, new Uint8Array(data), options));
        } else if (data instanceof Uint8Array) {
            return await rawOp(Deno.writeFile(filename, data, options));
        } else if (ArrayBuffer.isView(data)) {
            return await rawOp(Deno.writeFile(filename, bytes(data), options));
        } else if (data) {
            return await rawOp(Deno.writeFile(filename, data, options));
        }
    } else if (isNodeLike) {
        if (typeof Blob === "function" && data instanceof Blob) {
            const reader = data.stream();
            const writer = createNodeWritableStream(filename, options);
            await reader.pipeTo(writer);
        } else if (typeof ReadableStream === "function" && data instanceof ReadableStream) {
            const writer = createNodeWritableStream(filename, options);
            await data.pipeTo(writer);
        } else {
            const fs = await import("fs/promises");
            const { append, ...rest } = options;
            let _data: Uint8Array | string;

            if (data instanceof ArrayBuffer) {
                _data = new Uint8Array(data);
            } else if (data instanceof Uint8Array) {
                _data = data;
            } else if (ArrayBuffer.isView(data)) {
                _data = bytes(data);
            } else if (typeof data === "string") {
                _data = data;
            } else {
                throw new TypeError("Unsupported data type");
            }

            return await rawOp(fs.writeFile(filename, _data, {
                flag: append ? "a" : "w",
                ...rest,
            }));
        }
    } else {
        const handle = await getFileHandle(filename, { root: options.root, create: true });
        return await writeFileHandle(handle, data, options);
    }
}

async function writeFileHandle(
    handle: FileSystemFileHandle,
    data: string | ArrayBuffer | ArrayBufferView | ReadableStream<Uint8Array> | Blob,
    options: {
        append?: boolean;
        signal?: AbortSignal;
    }
): Promise<void> {
    const writer = await createFileHandleWritableStream(handle, options);

    if (options.signal) {
        const { signal } = options;

        if (signal.aborted) {
            throw signal.reason;
        } else {
            signal.addEventListener("abort", () => {
                writer.abort(signal.reason);
            });
        }
    }

    try {
        if (data instanceof Blob) {
            await data.stream().pipeTo(writer);
        } else if (data instanceof ReadableStream) {
            await data.pipeTo(writer);
        } else {
            await writer.write(data);
            await writer.close();
        }
    } catch (err) {
        throw wrapFsError(err, "file");
    }
}

/**
 * Writes multiple lines of content to the file.
 * 
 * This function will automatically detect the line ending of the current
 * content and use it to write the new lines. If the file is empty or does not
 * exists (will be created automatically), it will use the system's default line
 * ending to separate lines.
 * 
 * This function will append a new line at the end of the final content, in
 * appending mode, it will also prepend a line ending before the input lines if
 * the current content doesn't ends with one.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { writeLines } from "@ayonli/jsext/fs";
 * 
 * await writeLines("/path/to/file.txt", ["Hello", "World"]);
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { writeLines } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * await writeLines("/path/to/file.txt", ["Hello", "World"], { root });
 * ```
 * 
 * @example
 * ```ts
 * // append the lines to the file
 * import { writeLines } from "@ayonli/jsext/fs";
 * 
 * await writeLines("/path/to/file.txt", ["Hello", "World"], { append: true });
 * ```
 */
export async function writeLines(
    target: string | FileSystemFileHandle,
    lines: string[],
    options: WriteFileOptions = {}
): Promise<void> {
    const current = await readFileAsText(target, options).catch(err => {
        if (as(err, Exception)?.name !== "NotFoundError") {
            throw err;
        } else {
            return "";
        }
    });
    const lineEndings = current.match(/\r?\n/g);
    let eol = EOL;

    if (lineEndings) {
        const crlf = lineEndings.filter(e => e === "\r\n").length;
        const lf = lineEndings.length - crlf;
        eol = crlf > lf ? "\r\n" : "\n";
    }

    let content = lines.join(eol);

    if (!content.endsWith(eol)) {
        if (eol === "\r\n" && content.endsWith("\r")) {
            content += "\n";
        } else {
            content += eol;
        }
    }

    if (options.append && !current.endsWith(eol) && !content.startsWith(eol)) {
        if (eol === "\r\n" && current.endsWith("\r")) {
            if (!content.startsWith("\n")) {
                content = "\n" + content;
            }
        } else {
            content = eol + content;
        }
    }

    await writeFile(target, content, options);
}

/**
 * Truncates (or extends) the file to reach the specified `size`. If `size` is
 * not specified then the entire file contents are truncated.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { stat, truncate } from "@ayonli/jsext/fs";
 * 
 * await truncate("/path/to/file.txt", 1024);
 * const info = await stat("/path/to/file.txt");
 * console.assert(info.size === 1024);
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { stat, truncate } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * await truncate("/path/to/file.txt", 1024, { root });
 * const info = await stat("/path/to/file.txt", { root });
 * console.assert(info.size === 1024);
 * ```
 * 
 * @example
 * ```ts
 * // truncate the file to zero size
 * import { stat, truncate } from "@ayonli/jsext/fs";
 * 
 * await truncate("/path/to/file.txt");
 * const info = await stat("/path/to/file.txt");
 * console.assert(info.size === 0);
 * ```
 */
export async function truncate(
    target: string | FileSystemFileHandle,
    size = 0,
    options: FileSystemOptions = {}
): Promise<void> {
    if (typeof target === "object") {
        return await truncateFileHandle(target, size);
    }

    const filename = target;

    if (isDeno) {
        await rawOp(Deno.truncate(filename, size));
    } else if (isNodeLike) {
        const fs = await import("fs/promises");
        await rawOp(fs.truncate(filename, size));
    } else {
        const handle = await getFileHandle(filename, { root: options.root });
        await truncateFileHandle(handle, size);
    }
}

async function truncateFileHandle(
    handle: FileSystemFileHandle,
    size: number = 0
): Promise<void> {
    try {
        const writer = await handle.createWritable({ keepExistingData: true });
        await writer.truncate(size);
        await writer.close();
    } catch (err) {
        throw wrapFsError(err, "file");
    }
}

/**
 * Options for the {@link remove} function.
 */
export interface RemoveOptions extends FileSystemOptions {
    /**
     * Whether to delete the sub-directories and files recursively. This option
     * is required in order to remove a non-empty directory.
     */
    recursive?: boolean;
}

/**
 * Removes the file or directory of the given path from the file system.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { remove } from "@ayonli/jsext/fs";
 * 
 * await remove("/path/to/file.txt");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { remove } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * await remove("/path/to/file.txt", { root });
 * ```
 * 
 * @example
 * ```ts
 * // remove the directory and its contents recursively
 * import { remove } from "@ayonli/jsext/fs";
 * 
 * await remove("/path/to/dir", { recursive: true });
 * ```
 */
export async function remove(path: string, options: RemoveOptions = {}): Promise<void> {
    if (isDeno) {
        await rawOp(Deno.remove(path, options));
    } else if (isNodeLike) {
        const fs = await import("fs/promises");

        if (typeof fs.rm === "function") {
            await rawOp(fs.rm(path, options));
        } else {
            try {
                const _stat = await fs.stat(path);

                if (_stat.isDirectory()) {
                    await fs.rmdir(path, options);
                } else {
                    await fs.unlink(path);
                }
            } catch (err) {
                throw wrapFsError(err);
            }
        }
    } else {
        const parent = dirname(path);
        const name = basename(path);
        const dir = await getDirHandle(parent, { root: options.root });
        await rawOp(dir.removeEntry(name, options), "directory");
    }
}

/**
 * Renames the file or directory from the old path to the new path.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { rename } from "@ayonli/jsext/fs";
 * 
 * await rename("/path/to/old.txt", "/path/to/new.txt");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { rename } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * await rename("/path/to/old.txt", "/path/to/new.txt", { root });
 * ```
 */
export async function rename(
    oldPath: string,
    newPath: string,
    options: FileSystemOptions = {}
): Promise<void> {
    if (isDeno) {
        await rawOp(Deno.rename(oldPath, newPath));
    } else if (isNodeLike) {
        const fs = await import("fs/promises");
        await rawOp(fs.rename(oldPath, newPath));
    } else {
        return await copyInBrowser(oldPath, newPath, {
            root: options.root,
            recursive: true,
            move: true,
        });
    }
}

/**
 * Options for the {@link copy} function.
 */
export interface CopyOptions extends FileSystemOptions {
    /**
     * Whether to copy the directory recursively, which means to copy the
     * directory and all its contents.
     */
    recursive?: boolean;
}

/**
 * Copies the file or directory (and its contents) from the old location to the
 * new location.
 * 
 * NOTE: If the old location is a file and the new location is a directory, the
 * file will be copied into the new directory with the old name.
 * 
 * NOTE: In Unix/Linux systems, when using the `cp -R` command to copy a path
 * without an ending slash, the command will copy the directory itself into the
 * new path if the new path already exists. This function does not have this
 * behavior, it does not distinguish between a path with a trailing slash and a
 * path without it. So when copying a directory, this function always copy its
 * contents to the new path, whether the new path already exists or not.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { copy } from "@ayonli/jsext/fs";
 * 
 * await copy("/path/to/old.txt", "/path/to/new.txt");
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { copy } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * await copy("/path/to/old.txt", "/path/to/new.txt", { root });
 * ```
 * 
 * @example
 * ```ts
 * // copy a directory and its contents recursively
 * import { copy } from "@ayonli/jsext/fs";
 * 
 * await copy("/path/to/dir", "/path/to/new", { recursive: true });
 * ```
 * 
 * @example
 * ```ts
 * // copy a file to a directory
 * import { copy, exists } from "@ayonli/jsext/fs";
 * 
 * await copy("/path/to/file.txt", "/path/to/dir");
 * console.assert(await exists("/path/to/dir/file.txt"));
 * ```
 */
export async function copy(src: string, dest: string, options?: CopyOptions): Promise<void>;
/**
 * @example
 * ```ts
 * // copy a file from the device's file system to the browser's file system (Chromium only)
 * import { copy, getFileHandle } from "@ayonli/jsext/fs";
 * 
 * const file1 = await window.showOpenFilePicker();
 * const file2 = await getFileHandle("/path/to/file.txt");
 * 
 * await copy(file1[0], file2);
 * ```
 */
export async function copy(
    src: FileSystemFileHandle,
    dest: FileSystemFileHandle | FileSystemDirectoryHandle
): Promise<void>;
/**
 * @example
 * ```ts
 * // copy a directory from the device's file system to the browser's file system (Chromium only)
 * import { copy, getDirHandle } from "@ayonli/jsext/fs";
 * 
 * const dir1 = await window.showDirectoryPicker();
 * const dir2 = await getDirHandle("/path/to/dir");
 * 
 * await copy(dir1, dir2, { recursive: true });
 * ```
 */
export async function copy(
    src: FileSystemDirectoryHandle,
    dest: FileSystemDirectoryHandle,
    options?: Pick<CopyOptions, "recursive">
): Promise<void>;
export async function copy(
    src: string | FileSystemFileHandle | FileSystemDirectoryHandle,
    dest: string | FileSystemFileHandle | FileSystemDirectoryHandle,
    options: CopyOptions = {}
): Promise<void> {
    if (typeof src === "object" || typeof dest === "object") {
        return copyInBrowser(src, dest, { recursive: options?.recursive ?? false });
    }

    if (isDeno || isNodeLike) {
        const oldStat = await stat(src, { followSymlink: true });
        const isDirSrc = oldStat.kind === "directory";
        let isDirDest = false;

        if (isDirSrc && !options.recursive) {
            throw new Exception("Cannot copy a directory without the 'recursive' option", {
                name: "InvalidOperationError",
                code: 400,
            });
        }

        try {
            const newStat = await stat(dest, { followSymlink: true });
            isDirDest = newStat.kind === "directory";

            if (isDirSrc && !isDirDest) {
                throw new Exception(`'${dest}' is not a directory`, {
                    name: "NotDirectoryError",
                    code: 415,
                });
            }
        } catch {
            if (isDirSrc) {
                await mkdir(dest);
                isDirDest = true;
            }
        }

        if (isDeno) {
            if (isDirSrc) {
                const entries = readDir(src, { recursive: true });

                for await (const entry of entries) {
                    const _oldPath = join(src, entry.relativePath);
                    const _newPath = join(dest, entry.relativePath);

                    if (entry.kind === "directory") {
                        await rawOp(Deno.mkdir(_newPath));
                    } else {
                        await rawOp(Deno.copyFile(_oldPath, _newPath));
                    }
                }
            } else {
                const _newPath = isDirDest ? join(dest, basename(src)) : dest;
                await rawOp(Deno.copyFile(src, _newPath));
            }
        } else {
            const fs = await import("fs/promises");

            if (isDirSrc) {
                const entries = readDir(src, { recursive: true });

                for await (const entry of entries) {
                    const _oldPath = join(src, entry.relativePath);
                    const _newPath = join(dest, entry.relativePath);

                    if (entry.kind === "directory") {
                        await rawOp(fs.mkdir(_newPath));
                    } else {
                        await rawOp(fs.copyFile(_oldPath, _newPath));
                    }
                }
            } else {
                const _newPath = isDirDest ? join(dest, basename(src)) : dest;
                await rawOp(fs.copyFile(src, _newPath));
            }
        }
    } else {
        return await copyInBrowser(src, dest, {
            root: options.root,
            recursive: options.recursive ?? false,
        });
    }
}

async function copyInBrowser(
    src: string | FileSystemFileHandle | FileSystemDirectoryHandle,
    dest: string | FileSystemFileHandle | FileSystemDirectoryHandle,
    options: FileSystemOptions & {
        recursive?: boolean;
        move?: boolean;
    } = {}
): Promise<void> {
    if (typeof src === "object" && typeof dest !== "object") {
        throw new TypeError("The destination must be a FileSystemHandle");
    } else if (typeof dest === "object" && typeof src !== "object") {
        throw new TypeError("The source must be a FileSystemHandle");
    } else if (typeof src === "object" && typeof dest === "object") {
        if (src.kind === "file") {
            if (dest.kind === "file") {
                return await copyFileHandleToFileHandle(src, dest);
            } else {
                return await copyFileHandleToDirHandle(src, dest);
            }
        } else if (dest.kind === "directory") {
            if (!options.recursive) {
                throw new Exception("Cannot copy a directory without the 'recursive' option", {
                    name: "InvalidOperationError",
                    code: 400,
                });
            }

            return await copyDirHandleToDirHandle(src, dest);
        } else {
            throw new Exception("The destination location is not a directory", {
                name: "NotDirectoryError",
                code: 415,
            });
        }
    }

    const oldParent = dirname(src as string);
    const oldName = basename(src as string);

    let oldDir = await getDirHandle(oldParent, { root: options.root });
    const [oldErr, oldFile] = await _try(rawOp(oldDir.getFileHandle(oldName), "file"));

    if (oldFile) {
        const newParent = dirname(dest as string);
        const newName = basename(dest as string);
        let newDir = await getDirHandle(newParent, { root: options.root });
        const [newErr, newFile] = await _try(rawOp(newDir.getFileHandle(newName, {
            create: true,
        }), "file"));

        if (newFile) {
            await copyFileHandleToFileHandle(oldFile, newFile);

            if (options.move) {
                await rawOp(oldDir.removeEntry(oldName), "directory");
            }
        } else if (as(newErr, Exception)?.name === "IsDirectoryError" && !options.move) {
            // The destination is a directory, copy the file into the new path
            // with the old name.
            newDir = await rawOp(newDir.getDirectoryHandle(newName), "directory");
            await copyFileHandleToDirHandle(oldFile, newDir);
        } else {
            throw newErr;
        }
    } else if (as(oldErr, Exception)?.name === "IsDirectoryError") {
        if (!options.recursive) {
            throw new Exception("Cannot copy a directory without the 'recursive' option", {
                name: "InvalidOperationError",
                code: 400,
            });
        }

        const parent = oldDir;
        oldDir = await rawOp(oldDir.getDirectoryHandle(oldName), "directory");
        const newDir = await getDirHandle(dest as string, { root: options.root, create: true });

        await copyDirHandleToDirHandle(oldDir, newDir);

        if (options.move) {
            await rawOp(parent.removeEntry(oldName, { recursive: true }), "directory");
        }
    } else {
        throw oldErr;
    }
}

async function copyFileHandleToFileHandle(
    src: FileSystemFileHandle,
    dest: FileSystemFileHandle
) {
    try {
        const srcFile = await src.getFile();
        const destFile = await dest.createWritable();
        await srcFile.stream().pipeTo(destFile);
    } catch (err) {
        throw wrapFsError(err, "file");
    }
}

async function copyFileHandleToDirHandle(
    src: FileSystemFileHandle,
    dest: FileSystemDirectoryHandle
) {
    try {
        const srcFile = await src.getFile();
        const newFile = await dest.getFileHandle(src.name, { create: true });
        const destFile = await newFile.createWritable();

        await srcFile.stream().pipeTo(destFile);
    } catch (err) {
        throw wrapFsError(err, "file");
    }
}

async function copyDirHandleToDirHandle(
    src: FileSystemDirectoryHandle,
    dest: FileSystemDirectoryHandle
) {
    const entries = src.entries();

    for await (const [_, entry] of entries) {
        if (entry.kind === "file") {
            try {
                const oldFile = await (entry as FileSystemFileHandle).getFile();
                const newFile = await dest.getFileHandle(entry.name, {
                    create: true,
                });
                const reader = oldFile.stream();
                const writer = await newFile.createWritable();

                await reader.pipeTo(writer);
            } catch (err) {
                throw wrapFsError(err, "file");
            }
        } else {
            const newSubDir = await rawOp(dest.getDirectoryHandle(entry.name, {
                create: true,
            }), "directory");
            await copyDirHandleToDirHandle(entry as FileSystemDirectoryHandle, newSubDir);
        }
    }
}

/**
 * Options for the {@link link} function.
 */
export interface LinkOptions {
    /**
     * Create a symbolic link instead of a hard link.
     */
    symbolic?: boolean;
}

/**
 * Creates a hard link (or symbolic link) from the source path to the destination
 * path.
 * 
 * NOTE: This function is only available in Node.js, Deno and Bun.
 * 
 * @example
 * ```ts
 * // create a hard link
 * import { link } from "@ayonli/jsext/fs";
 * 
 * await link("/path/to/file.txt", "/path/to/link.txt");
 * ```
 * 
 * @example
 * ```ts
 * // create a symbolic link
 * import { link } from "@ayonli/jsext/fs";
 * 
 * await link("/path/to/file.txt", "/path/to/link.txt", { symbolic: true });
 * ```
 */
export async function link(src: string, dest: string, options: LinkOptions = {}): Promise<void> {
    if (isDeno) {
        if (options.symbolic) {
            if (platform() === "windows") {
                const _stat = await stat(src);
                await rawOp(Deno.symlink(src, dest, {
                    type: _stat.kind === "directory" ? "dir" : "file",
                }));
            } else {
                await rawOp(Deno.symlink(src, dest));
            }
        } else {
            await rawOp(Deno.link(src, dest));
        }
    } else if (isNodeLike) {
        const fs = await import("fs/promises");

        if (options.symbolic) {
            if (platform() === "windows") {
                const _stat = await stat(src);
                await rawOp(fs.symlink(src, dest, _stat.kind === "directory" ? "dir" : "file"));
            } else {
                await rawOp(fs.symlink(src, dest));
            }
        } else {
            await rawOp(fs.link(src, dest));
        }
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Returns the destination path of a symbolic link.
 * 
 * NOTE: This function is only available in Node.js, Deno and Bun.
 * 
 * @example
 * ```ts
 * import { readLink } from "@ayonli/jsext/fs";
 * 
 * const dest = await readLink("/path/to/link.txt");
 * console.log(dest);
 * ```
 */
export async function readLink(path: string): Promise<string> {
    if (isDeno) {
        return await rawOp(Deno.readLink(path));
    } else if (isNodeLike) {
        const fs = await import("fs/promises");
        return await rawOp(fs.readlink(path));
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Changes the permission of the specified file or directory.
 * 
 * The mode is a sequence of 3 octal numbers. The first/left-most number
 * specifies the permissions for the owner. The second number specifies the
 * permissions for the group. The last/right-most number specifies the
 * permissions for others. For example, with a mode of 0o764, the owner (7) can
 * read/write/execute, the group (6) can read/write and everyone else (4) can
 * read only.
 * 
 * | Number | Description |
 * | ------ | ----------- |
 * | 7      | read, write, and execute |
 * | 6      | read and write |
 * | 5      | read and execute |
 * | 4      | read only |
 * | 3      | write and execute |
 * | 2      | write only |
 * | 1      | execute only |
 * | 0      | no permission |
 * 
 * NOTE: This function is only available in Node.js, Deno and Bun, and only
 * works in Unix/Linux systems, in other environments, it's a no-op.
 * 
 * @example
 * ```ts
 * import { chmod } from "@ayonli/jsext/fs";
 * 
 * // Change the file's permission to read/write for owner, read for group and others.
 * await chmod("/path/to/file.txt", 0o644);
 * ```
 */
export async function chmod(path: string, mode: number): Promise<void> {
    if (platform() !== "windows") {
        if (isDeno) {
            await rawOp(Deno.chmod(path, mode));
        } else if (isNodeLike) {
            const fs = await import("fs/promises");
            await rawOp(fs.chmod(path, mode));
        }
    }
}

/**
 * Changes the owner and group of the specified file or directory.
 * 
 * NOTE: This function is only available in Node.js, Deno and Bun, and only
 * works in Unix/Linux systems, in other environments, it's a no-op.
 * 
 * @example
 * ```ts
 * import { chown } from "@ayonli/jsext/fs";
 * 
 * // Change the owner and group of the file to root.
 * await chown("/path/to/file.txt", 0, 0);
 * ```
 */
export async function chown(path: string, uid: number, gid: number): Promise<void> {
    if (platform() !== "windows") {
        if (isDeno) {
            await rawOp(Deno.chown(path, uid, gid));
        } else if (isNodeLike) {
            const fs = await import("fs/promises");
            await rawOp(fs.chown(path, uid, gid));
        }
    }
}

/**
 * Changes the access (`atime`) and modification (`mtime`) times of the file
 * or directory. Given times are either in seconds (UNIX epoch time) or as `Date`
 * objects.
 * 
 * NOTE: This function only works in Node.js, Deno and Bun, in other
 * environments, it's a no-op.
 * 
 * @example
 * ```ts
 * import { utimes } from "@ayonli/jsext/fs";
 * 
 * // Set the access and modification times to the current time.
 * await utimes("/path/to/file.txt", Date.now(), Date.now());
 * ```
 */
export async function utimes(
    path: string,
    atime: number | Date,
    mtime: number | Date
): Promise<void> {
    if (isDeno) {
        await rawOp(Deno.utime(path, atime, mtime));
    } else if (isNodeLike) {
        const fs = await import("fs/promises");
        await rawOp(fs.utimes(path, atime, mtime));
    }
}

/**
 * Creates a readable stream for the target file.
 * 
 * NOTE: In Node.js, this function requires Node.js v18.0 or above.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { createReadableStream } from "@ayonli/jsext/fs";
 * import { readAsText } from "@ayonli/jsext/reader";
 * 
 * const input = createReadableStream("/path/to/file.txt");
 * 
 * const text = await readAsText(input);
 * console.log(text);
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { createReadableStream } from "@ayonli/jsext/fs";
 * import { readAsText } from "@ayonli/jsext/reader";
 * 
 * const root = await window.showDirectoryPicker();
 * const input = createReadableStream("/path/to/file.txt", { root });
 * 
 * const text = await readAsText(input);
 * console.log(text);
 * ```
 */
export function createReadableStream(
    target: string | FileSystemFileHandle,
    options: FileSystemOptions = {}
): ReadableStream<Uint8Array> {
    if (isNodeLike) {
        if (typeof target === "object") {
            throw new TypeError("Expected a file path, got a file handle");
        }

        const filename = target as string;
        let reader: import("fs").ReadStream;

        return new ReadableStream<Uint8Array>({
            async start(controller) {
                const fs = await import("fs");

                reader = fs.createReadStream(filename);
                reader.on("data", (chunk: Buffer) => {
                    const bytes = new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength);
                    controller.enqueue(bytes);
                });
                reader.on("end", () => controller.close());
                reader.on("error", (err: Error) => controller.error(err));
            },
            cancel(reason = undefined) {
                reader.destroy(reason);
            },
        });
    }

    return resolveByteStream((async () => {
        if (typeof target === "object") {
            return await readFileHandleAsStream(target);
        }

        const filename = target;

        if (isDeno) {
            const file = await rawOp(Deno.open(filename, { read: true }));
            return file.readable;
        } else {
            const handle = await getFileHandle(filename, { root: options.root });
            return await readFileHandleAsStream(handle);
        }
    })());
}

async function readFileHandleAsStream(
    handle: FileSystemFileHandle
): Promise<ReadableStream<Uint8Array>> {
    const file = await rawOp(handle.getFile(), "file");
    return file.stream();
}

/**
 * @deprecated use `createReadableStream` instead.
 */
export const readFileAsStream = createReadableStream;

/**
 * Creates a writable stream for the target file.
 * 
 * NOTE: In Node.js, this function requires Node.js v18.0 or above.
 * 
 * @example
 * ```ts
 * // with the default storage
 * import { createWritableStream } from "@ayonli/jsext/fs";
 * 
 * const output = createWritableStream("/path/to/file.txt");
 * const res = await fetch("https://example.com/file.txt");
 * 
 * await res.body!.pipeTo(output);
 * ```
 * 
 * @example
 * ```ts
 * // with a user-selected directory as root (Chromium only)
 * import { createWritableStream } from "@ayonli/jsext/fs";
 * 
 * const root = await window.showDirectoryPicker();
 * const output = createWritableStream("/path/to/file.txt", { root });
 * const res = await fetch("https://example.com/file.txt");
 * 
 * await res.body!.pipeTo(output);
 * ```
 */
export function createWritableStream(
    target: string | FileSystemFileHandle,
    options: Omit<WriteFileOptions, "signal"> = {}
): WritableStream<Uint8Array> {
    if (typeof target === "object") {
        const { readable, writable } = new TransformStream();
        createFileHandleWritableStream(target, options)
            .then(stream => readable.pipeTo(stream));
        return writable;
    }

    const filename = target;

    if (isDeno) {
        const { readable, writable } = new TransformStream();
        Deno.open(filename, { write: true, create: true, append: options.append ?? false })
            .then(file => file.writable)
            .then(stream => readable.pipeTo(stream));
        return writable;
    } else if (isNodeLike) {
        return createNodeWritableStream(filename, options);
    } else {
        const { readable, writable } = new TransformStream();
        getFileHandle(filename, { root: options.root, create: true })
            .then(handle => createFileHandleWritableStream(handle, options))
            .then(stream => readable.pipeTo(stream));
        return writable;
    }
}

async function createFileHandleWritableStream(handle: FileSystemFileHandle, options: {
    append?: boolean;
}): Promise<FileSystemWritableFileStream> {
    const stream = await rawOp(handle.createWritable({
        keepExistingData: options?.append ?? false,
    }), "file");

    if (options.append) {
        const file = await rawOp(handle.getFile(), "file");
        file.size && stream.seek(file.size);
    }

    return stream;
}

function createNodeWritableStream(filename: string, options: {
    append?: boolean;
    mode?: number;
}): WritableStream<Uint8Array> {
    let dest: import("fs").WriteStream;
    return new WritableStream<Uint8Array>({
        async start() {
            const { append, ...rest } = options;
            const { createWriteStream } = await import("fs");
            dest = createWriteStream(filename, {
                flags: append ? "a" : "w",
                ...rest,
            });
        },
        write(chunk) {
            return new Promise<void>((resolve, reject) => {
                dest.write(chunk, (err) => err ? reject(err) : resolve());
            });
        },
        close() {
            return new Promise((resolve) => {
                dest.end(() => resolve());
            });
        },
        abort(reason) {
            dest.destroy(reason);
        }
    });
}
