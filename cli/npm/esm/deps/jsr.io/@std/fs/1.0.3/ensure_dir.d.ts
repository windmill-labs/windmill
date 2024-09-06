/**
 * Asynchronously ensures that the directory exists, like
 * {@linkcode https://www.ibm.com/docs/en/aix/7.3?topic=m-mkdir-command#mkdir__row-d3e133766 | mkdir -p}.
 *
 * If the directory already exists, this function does nothing. If the directory
 * does not exist, it is created.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param dir The path of the directory to ensure, as a string or URL.
 *
 * @returns A promise that resolves once the directory exists.
 *
 * @example Usage
 * ```ts no-eval
 * import { ensureDir } from "@std/fs/ensure-dir";
 *
 * await ensureDir("./bar");
 * ```
 */
export declare function ensureDir(dir: string | URL): Promise<void>;
/**
 * Synchronously ensures that the directory exists, like
 * {@linkcode https://www.ibm.com/docs/en/aix/7.3?topic=m-mkdir-command#mkdir__row-d3e133766 | mkdir -p}.
 *
 * If the directory already exists, this function does nothing. If the directory
 * does not exist, it is created.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param dir The path of the directory to ensure, as a string or URL.
 *
 * @returns A void value that returns once the directory exists.
 *
 * @example Usage
 * ```ts no-eval
 * import { ensureDirSync } from "@std/fs/ensure-dir";
 *
 * ensureDirSync("./bar");
 * ```
 */
export declare function ensureDirSync(dir: string | URL): void;
//# sourceMappingURL=ensure_dir.d.ts.map