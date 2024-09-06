// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import * as dntShim from "../../../../../_dnt.shims.js";
import { getFileInfoType } from "./_get_file_info_type.js";
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
export async function ensureDir(dir) {
    try {
        const fileInfo = await dntShim.Deno.stat(dir);
        throwIfNotDirectory(fileInfo);
        return;
    }
    catch (err) {
        if (!(err instanceof dntShim.Deno.errors.NotFound)) {
            throw err;
        }
    }
    // The dir doesn't exist. Create it.
    // This can be racy. So we catch AlreadyExists and check stat again.
    try {
        await dntShim.Deno.mkdir(dir, { recursive: true });
    }
    catch (err) {
        if (!(err instanceof dntShim.Deno.errors.AlreadyExists)) {
            throw err;
        }
        const fileInfo = await dntShim.Deno.stat(dir);
        throwIfNotDirectory(fileInfo);
    }
}
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
export function ensureDirSync(dir) {
    try {
        const fileInfo = dntShim.Deno.statSync(dir);
        throwIfNotDirectory(fileInfo);
        return;
    }
    catch (err) {
        if (!(err instanceof dntShim.Deno.errors.NotFound)) {
            throw err;
        }
    }
    // The dir doesn't exist. Create it.
    // This can be racy. So we catch AlreadyExists and check stat again.
    try {
        dntShim.Deno.mkdirSync(dir, { recursive: true });
    }
    catch (err) {
        if (!(err instanceof dntShim.Deno.errors.AlreadyExists)) {
            throw err;
        }
        const fileInfo = dntShim.Deno.statSync(dir);
        throwIfNotDirectory(fileInfo);
    }
}
function throwIfNotDirectory(fileInfo) {
    if (!fileInfo.isDirectory) {
        throw new Error(`Failed to ensure directory exists: expected 'dir', got '${getFileInfoType(fileInfo)}'`);
    }
}
