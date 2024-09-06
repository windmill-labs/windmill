/**
 * Asynchronously ensures that a directory is empty.
 *
 * If the directory does not exist, it is created. The directory itself is not
 * deleted.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param dir The path of the directory to empty, as a string or URL.
 *
 * @returns A void promise that resolves once the directory is empty.
 *
 * @example Usage
 * ```ts no-eval
 * import { emptyDir } from "@std/fs/empty-dir";
 *
 * await emptyDir("./foo");
 * ```
 */
export declare function emptyDir(dir: string | URL): Promise<void>;
/**
 * Synchronously ensures that a directory is empty deletes the directory
 * contents it is not empty.
 *
 * If the directory does not exist, it is created. The directory itself is not
 * deleted.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param dir The path of the directory to empty, as a string or URL.
 *
 * @returns A void value that returns once the directory is empty.
 *
 * @example Usage
 * ```ts no-eval
 * import { emptyDirSync } from "@std/fs/empty-dir";
 *
 * emptyDirSync("./foo");
 * ```
 */
export declare function emptyDirSync(dir: string | URL): void;
//# sourceMappingURL=empty_dir.d.ts.map