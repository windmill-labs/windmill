/** Options for {@linkcode move} and {@linkcode moveSync}. */
export interface MoveOptions {
    /**
     * Whether the destination file should be overwritten if it already exists.
     *
     * @default {false}
     */
    overwrite?: boolean;
}
/**
 * Asynchronously moves a file or directory (along with its contents).
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param src The source file or directory as a string or URL.
 * @param dest The destination file or directory as a string or URL.
 * @param options Options for the move operation.
 * @throws {Deno.errors.AlreadyExists} If `dest` already exists and
 * `options.overwrite` is `false`.
 * @throws {Deno.errors.NotSupported} If `src` is a sub-directory of `dest`.
 *
 * @returns A void promise that resolves once the operation completes.
 *
 * @example Basic usage
 * ```ts no-eval
 * import { move } from "@std/fs/move";
 *
 * await move("./foo", "./bar");
 * ```
 *
 * This will move the file or directory at `./foo` to `./bar` without
 * overwriting.
 *
 * @example Overwriting
 * ```ts no-eval
 * import { move } from "@std/fs/move";
 *
 * await move("./foo", "./bar", { overwrite: true });
 * ```
 *
 * This will move the file or directory at `./foo` to `./bar`, overwriting
 * `./bar` if it already exists.
 */
export declare function move(src: string | URL, dest: string | URL, options?: MoveOptions): Promise<void>;
/**
 * Synchronously moves a file or directory (along with its contents).
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param src The source file or directory as a string or URL.
 * @param dest The destination file or directory as a string or URL.
 * @param options Options for the move operation.
 * @throws {Deno.errors.AlreadyExists} If `dest` already exists and
 * `options.overwrite` is `false`.
 * @throws {Deno.errors.NotSupported} If `src` is a sub-directory of `dest`.
 *
 * @returns A void value that returns once the operation completes.
 *
 * @example Basic usage
 * ```ts no-eval
 * import { moveSync } from "@std/fs/move";
 *
 * moveSync("./foo", "./bar");
 * ```
 *
 * This will move the file or directory at `./foo` to `./bar` without
 * overwriting.
 *
 * @example Overwriting
 * ```ts no-eval
 * import { moveSync } from "@std/fs/move";
 *
 * moveSync("./foo", "./bar", { overwrite: true });
 * ```
 *
 * This will move the file or directory at `./foo` to `./bar`, overwriting
 * `./bar` if it already exists.
 */
export declare function moveSync(src: string | URL, dest: string | URL, options?: MoveOptions): void;
//# sourceMappingURL=move.d.ts.map