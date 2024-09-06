/**
 * Asynchronously ensures that the file exists.
 *
 * If the file already exists, this function does nothing. If the parent
 * directories for the file do not exist, they are created.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param filePath The path of the file to ensure, as a string or URL.
 *
 * @returns A void promise that resolves once the file exists.
 *
 * @example Usage
 * ```ts no-eval
 * import { ensureFile } from "@std/fs/ensure-file";
 *
 * await ensureFile("./folder/targetFile.dat");
 * ```
 */
export declare function ensureFile(filePath: string | URL): Promise<void>;
/**
 * Synchronously ensures that the file exists.
 *
 * If the file already exists, this function does nothing. If the parent
 * directories for the file do not exist, they are created.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param filePath The path of the file to ensure, as a string or URL.
 *
 * @returns A void value that returns once the file exists.
 *
 * @example Usage
 * ```ts no-eval
 * import { ensureFileSync } from "@std/fs/ensure-file";
 *
 * ensureFileSync("./folder/targetFile.dat");
 * ```
 */
export declare function ensureFileSync(filePath: string | URL): void;
//# sourceMappingURL=ensure_file.d.ts.map