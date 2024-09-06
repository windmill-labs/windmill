// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Documentation and interface for walk were adapted from Go
// https://golang.org/pkg/path/filepath/#Walk
// Copyright 2009 The Go Authors. All rights reserved. BSD license.
import * as dntShim from "../../../../../_dnt.shims.js";
import { join } from "../../path/1.0.4/join.js";
import { toPathString } from "./_to_path_string.js";
import { createWalkEntry, createWalkEntrySync, } from "./_create_walk_entry.js";
function include(path, exts, match, skip) {
    if (exts && !exts.some((ext) => path.endsWith(ext))) {
        return false;
    }
    if (match && !match.some((pattern) => !!path.match(pattern))) {
        return false;
    }
    if (skip && skip.some((pattern) => !!path.match(pattern))) {
        return false;
    }
    return true;
}
/**
 * Recursively walks through a directory and yields information about each file
 * and directory encountered.
 *
 * The root path determines whether the file paths are relative or absolute.
 * The root directory is included in the yielded entries.
 *
 * Requires `--allow-read` permission.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param root The root directory to start the walk from, as a string or URL.
 * @param options The options for the walk.
 * @throws {Deno.errors.NotFound} If the root directory does not exist.
 *
 * @returns An async iterable iterator that yields the walk entry objects.
 *
 * @example Basic usage
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo.ts
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk("."));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "foo.ts",
 * //     name: "foo.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Maximum file depth
 *
 * Setting the `maxDepth` option to `1` will only include the root directory and
 * its immediate children.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo
 *     └── bar.ts
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { maxDepth: 1 }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "foo",
 * //     name: "foo",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Exclude files
 *
 * Setting the `includeFiles` option to `false` will exclude files.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { includeFiles: false }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "foo",
 * //     name: "foo",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false,
 * //   },
 * // ]
 * ```
 *
 * @example Exclude directories
 *
 * Setting the `includeDirs` option to `false` will exclude directories.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { includeDirs: false }));
 * // [
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Exclude symbolic links
 *
 * Setting the `includeSymlinks` option to `false` will exclude symbolic links.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * ├── foo
 * └── link -> script.ts (symbolic link)
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { includeSymlinks: false }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Follow symbolic links
 *
 * Setting the `followSymlinks` option to `true` will follow symbolic links,
 * affecting the `path` property of the walk entry.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── link -> script.ts (symbolic link)
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { followSymlinks: true }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "link",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: true
 * //   },
 * // ]
 * ```
 *
 * @example Canonicalize symbolic links
 *
 * Setting the `canonicalize` option to `false` will canonicalize the path of
 * the followed symbolic link. Meaning, the `path` property of the walk entry
 * will be the path of the symbolic link itself.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── link -> script.ts (symbolic link)
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { followSymlinks: true, canonicalize: true }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "link",
 * //     name: "link",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: true
 * //   },
 * // ]
 * ```
 *
 * @example Filter by file extensions
 *
 * Setting the `exts` option to `[".ts"]` or `["ts"]` will only include entries
 * with the `.ts` file extension.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo.js
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { exts: [".ts"] }));
 * // [
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Filter by regular expressions
 *
 * Setting the `match` option to `[/.s/]` will only include entries with the
 * letter `s` in their name.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── README.md
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { match: [/s/] }));
 * // [
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Exclude by regular expressions
 *
 * Setting the `skip` option to `[/.s/]` will exclude entries with the letter
 * `s` in their name.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── README.md
 * ```
 *
 * ```ts no-eval
 * import { walk } from "@std/fs/walk";
 *
 * await Array.fromAsync(walk(".", { skip: [/s/] }));
 * // [
 * //   {
 * //     path: "README.md",
 * //     name: "README.md",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 */
export async function* walk(root, options) {
    let { maxDepth = Infinity, includeFiles = true, includeDirs = true, includeSymlinks = true, followSymlinks = false, canonicalize = true, exts = undefined, match = undefined, skip = undefined, } = options ?? {};
    if (maxDepth < 0) {
        return;
    }
    root = toPathString(root);
    if (exts) {
        exts = exts.map((ext) => ext.startsWith(".") ? ext : `.${ext}`);
    }
    if (includeDirs && include(root, exts, match, skip)) {
        yield await createWalkEntry(root);
    }
    if (maxDepth < 1 || !include(root, undefined, undefined, skip)) {
        return;
    }
    for await (const entry of dntShim.Deno.readDir(root)) {
        let path = join(root, entry.name);
        let { isSymlink, isDirectory } = entry;
        if (isSymlink) {
            if (!followSymlinks) {
                if (includeSymlinks && include(path, exts, match, skip)) {
                    yield { path, ...entry };
                }
                continue;
            }
            const realPath = await dntShim.Deno.realPath(path);
            if (canonicalize) {
                path = realPath;
            }
            // Caveat emptor: don't assume |path| is not a symlink. realpath()
            // resolves symlinks but another process can replace the file system
            // entity with a different type of entity before we call lstat().
            ({ isSymlink, isDirectory } = await dntShim.Deno.lstat(realPath));
        }
        if (isSymlink || isDirectory) {
            const opts = {
                maxDepth: maxDepth - 1,
                includeFiles,
                includeDirs,
                includeSymlinks,
                followSymlinks,
            };
            if (exts !== undefined) {
                opts.exts = exts;
            }
            if (match !== undefined) {
                opts.match = match;
            }
            if (skip !== undefined) {
                opts.skip = skip;
            }
            yield* walk(path, opts);
        }
        else if (includeFiles && include(path, exts, match, skip)) {
            yield { path, ...entry };
        }
    }
}
/**
 * Recursively walks through a directory and yields information about each file
 * and directory encountered.
 *
 * The root path determines whether the file paths is relative or absolute.
 * The root directory is included in the yielded entries.
 *
 * Requires `--allow-read` permission.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param root The root directory to start the walk from, as a string or URL.
 * @param options The options for the walk.
 *
 * @returns A synchronous iterable iterator that yields the walk entry objects.
 *
 * @example Basic usage
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo.ts
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync("."));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "foo.ts",
 * //     name: "foo.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Maximum file depth
 *
 * Setting the `maxDepth` option to `1` will only include the root directory and
 * its immediate children.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo
 *     └── bar.ts
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { maxDepth: 1 }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "foo",
 * //     name: "foo",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Exclude files
 *
 * Setting the `includeFiles` option to `false` will exclude files.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { includeFiles: false }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "foo",
 * //     name: "foo",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false,
 * //   },
 * // ]
 * ```
 *
 * @example Exclude directories
 *
 * Setting the `includeDirs` option to `false` will exclude directories.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { includeDirs: false }));
 * // [
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Exclude symbolic links
 *
 * Setting the `includeSymlinks` option to `false` will exclude symbolic links.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * ├── foo
 * └── link -> script.ts (symbolic link)
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { includeSymlinks: false }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Follow symbolic links
 *
 * Setting the `followSymlinks` option to `true` will follow symbolic links,
 * affecting the `path` property of the walk entry.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── link -> script.ts (symbolic link)
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { followSymlinks: true }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "link",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: true
 * //   },
 * // ]
 * ```
 *
 * @example Canonicalize symbolic links
 *
 * Setting the `canonicalize` option to `false` will canonicalize the path of
 * the followed symbolic link. Meaning, the `path` property of the walk entry
 * will be the path of the symbolic link itself.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── link -> script.ts (symbolic link)
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { followSymlinks: true, canonicalize: true }));
 * // [
 * //   {
 * //     path: ".",
 * //     name: ".",
 * //     isFile: false,
 * //     isDirectory: true,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * //   {
 * //     path: "link",
 * //     name: "link",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: true
 * //   },
 * // ]
 * ```
 *
 * @example Filter by file extensions
 *
 * Setting the `exts` option to `[".ts"]` or `["ts"]` will only include entries
 * with the `.ts` file extension.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo.js
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { exts: [".ts"] }));
 * // [
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Filter by regular expressions
 *
 * Setting the `match` option to `[/.s/]` will only include entries with the
 * letter `s` in their name.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── README.md
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { match: [/s/] }));
 * // [
 * //   {
 * //     path: "script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 *
 * @example Exclude by regular expressions
 *
 * Setting the `skip` option to `[/.s/]` will exclude entries with the letter
 * `s` in their name.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── README.md
 * ```
 *
 * ```ts no-eval
 * import { walkSync } from "@std/fs/walk";
 *
 * Array.from(walkSync(".", { skip: [/s/] }));
 * // [
 * //   {
 * //     path: "README.md",
 * //     name: "README.md",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false
 * //   },
 * // ]
 * ```
 */
export function* walkSync(root, options) {
    let { maxDepth = Infinity, includeFiles = true, includeDirs = true, includeSymlinks = true, followSymlinks = false, canonicalize = true, exts = undefined, match = undefined, skip = undefined, } = options ?? {};
    root = toPathString(root);
    if (exts) {
        exts = exts.map((ext) => ext.startsWith(".") ? ext : `.${ext}`);
    }
    if (maxDepth < 0) {
        return;
    }
    if (includeDirs && include(root, exts, match, skip)) {
        yield createWalkEntrySync(root);
    }
    if (maxDepth < 1 || !include(root, undefined, undefined, skip)) {
        return;
    }
    const entries = dntShim.Deno.readDirSync(root);
    for (const entry of entries) {
        let path = join(root, entry.name);
        let { isSymlink, isDirectory } = entry;
        if (isSymlink) {
            if (!followSymlinks) {
                if (includeSymlinks && include(path, exts, match, skip)) {
                    yield { path, ...entry };
                }
                continue;
            }
            const realPath = dntShim.Deno.realPathSync(path);
            if (canonicalize) {
                path = realPath;
            }
            // Caveat emptor: don't assume |path| is not a symlink. realpath()
            // resolves symlinks but another process can replace the file system
            // entity with a different type of entity before we call lstat().
            ({ isSymlink, isDirectory } = dntShim.Deno.lstatSync(realPath));
        }
        if (isSymlink || isDirectory) {
            const opts = {
                maxDepth: maxDepth - 1,
                includeFiles,
                includeDirs,
                includeSymlinks,
                followSymlinks,
            };
            if (exts !== undefined) {
                opts.exts = exts;
            }
            if (match !== undefined) {
                opts.match = match;
            }
            if (skip !== undefined) {
                opts.skip = skip;
            }
            yield* walkSync(path, opts);
        }
        else if (includeFiles && include(path, exts, match, skip)) {
            yield { path, ...entry };
        }
    }
}
