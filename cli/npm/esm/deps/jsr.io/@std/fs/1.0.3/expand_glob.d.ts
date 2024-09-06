import { type GlobOptions } from "../../path/1.0.4/glob_to_regexp.js";
import { type WalkEntry } from "./_create_walk_entry.js";
export type { GlobOptions, WalkEntry };
/** Options for {@linkcode expandGlob} and {@linkcode expandGlobSync}. */
export interface ExpandGlobOptions extends Omit<GlobOptions, "os"> {
    /**
     * File path where to expand from.
     *
     * @default {Deno.cwd()}
     */
    root?: string;
    /**
     * List of glob patterns to be excluded from the expansion.
     *
     * @default {[]}
     */
    exclude?: string[];
    /**
     * Whether to include directories in entries.
     *
     * @default {true}
     */
    includeDirs?: boolean;
    /**
     * Whether to follow symbolic links.
     *
     * @default {false}
     */
    followSymlinks?: boolean;
    /**
     * Indicates whether the followed symlink's path should be canonicalized.
     * This option works only if `followSymlinks` is not `false`.
     *
     * @default {true}
     */
    canonicalize?: boolean;
}
/**
 * Returns an async iterator that yields each file path matching the given glob
 * pattern.
 *
 * The file paths are absolute paths. If `root` is not provided, the current
 * working directory is used. The `root` directory is not included in the
 * yielded file paths.
 *
 * Requires `--allow-read` permission.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param glob The glob pattern to expand.
 * @param options Additional options for the expansion.
 *
 * @returns An async iterator that yields each walk entry matching the glob
 * pattern.
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
 * // script.ts
 * import { expandGlob } from "@std/fs/expand-glob";
 *
 * await Array.fromAsync(expandGlob("*.ts"));
 * // [
 * //   {
 * //     path: "/Users/user/folder/script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false,
 * //   },
 * //   {
 * //     path: "/Users/user/folder/foo.ts",
 * //     name: "foo.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false,
 * //   },
 * // ]
 * ```
 *
 * @example Define root directory
 *
 * Setting the `root` option to `/folder` will expand the glob pattern from the
 * `/folder` directory.
 *
 * File structure:
 * ```
 * folder
 * ├── subdir
 * │   └── bar.ts
 * ├── script.ts
 * └── foo.ts
 * ```
 *
 * ```ts no-eval
 * // script.ts
 * import { expandGlob } from "@std/fs/expand-glob";
 *
 * await Array.fromAsync(expandGlob("*.ts", { root: "./subdir" }));
 * // [
 * //   {
 * //     path: "/Users/user/folder/subdir/bar.ts",
 * //     name: "bar.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false,
 * //   },
 * // ]
 * ```
 *
 * @example Exclude files
 *
 * Setting the `exclude` option to `["foo.ts"]` will exclude the `foo.ts` file
 * from the expansion.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo.ts
 * ```
 *
 * ```ts no-eval
 * // script.ts
 * import { expandGlob } from "@std/fs/expand-glob";
 *
 * await Array.fromAsync(expandGlob("*.ts", { exclude: ["foo.ts"] }));
 * // [
 * //   {
 * //     path: "/Users/user/folder/script.ts",
 * //     name: "true.ts",
 * //     isFile: false,
 * //     isDirectory: false,
 * //     isSymlink: false,
 * //   },
 * // ]
 * ```
 *
 * @example Exclude directories
 *
 * Setting the `includeDirs` option to `false` will exclude directories from the
 * expansion.
 *
 * File structure:
 * ```
 * folder
 * ├── subdir
 * │   └── bar.ts
 * ├── script.ts
 * └── foo.ts
 * ```
 *
 * ```ts no-eval
 * // script.ts
 * import { expandGlob } from "@std/fs/expand-glob";
 *
 * await Array.fromAsync(expandGlob("*", { includeDirs: false }));
 * // [
 * //   {
 * //     path: "/Users/user/folder/script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false,
 * //   },
 * //   {
 * //     path: "/Users/user/folder/foo.ts",
 * //     name: "foo.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false,
 * //   },
 * // ]
 * ```
 *
 * @example Follow symbolic links
 *
 * Setting the `followSymlinks` option to `true` will follow symbolic links.
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── link.ts -> script.ts (symbolic link)
 * ```
 *
 * ```ts no-eval
 * // script.ts
 * import { expandGlob } from "@std/fs/expand-glob";
 *
 * await Array.fromAsync(expandGlob("*.ts", { followSymlinks: true }));
 * // [
 * //   {
 * //     path: "/Users/user/folder/script.ts",
 * //     name: "script.ts",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: false,
 * //   },
 * //   {
 * //     path: "/Users/user/folder/symlink",
 * //     name: "symlink",
 * //     isFile: true,
 * //     isDirectory: false,
 * //     isSymlink: true,
 * //   },
 * // ]
 * ```
 */
export declare function expandGlob(glob: string | URL, options?: ExpandGlobOptions): AsyncIterableIterator<WalkEntry>;
/**
 * Returns an iterator that yields each file path matching the given glob
 * pattern. The file paths are relative to the provided `root` directory.
 * If `root` is not provided, the current working directory is used.
 * The `root` directory is not included in the yielded file paths.
 *
 * Requires the `--allow-read` flag.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param glob The glob pattern to expand.
 * @param options Additional options for the expansion.
 *
 * @returns An iterator that yields each walk entry matching the glob pattern.
 *
 * @example Usage
 *
 * File structure:
 * ```
 * folder
 * ├── script.ts
 * └── foo.ts
 * ```
 *
 * ```ts no-eval
 * // script.ts
 * import { expandGlobSync } from "@std/fs/expand-glob";
 *
 * const entries = [];
 * for (const entry of expandGlobSync("*.ts")) {
 *   entries.push(entry);
 * }
 *
 * entries[0]!.path; // "/Users/user/folder/script.ts"
 * entries[0]!.name; // "script.ts"
 * entries[0]!.isFile; // false
 * entries[0]!.isDirectory; // true
 * entries[0]!.isSymlink; // false
 *
 * entries[1]!.path; // "/Users/user/folder/foo.ts"
 * entries[1]!.name; // "foo.ts"
 * entries[1]!.isFile; // true
 * entries[1]!.isDirectory; // false
 * entries[1]!.isSymlink; // false
 * ```
 */
export declare function expandGlobSync(glob: string | URL, options?: ExpandGlobOptions): IterableIterator<WalkEntry>;
//# sourceMappingURL=expand_glob.d.ts.map