/**
 * Get directory list.
 *
 * @internal
 * @param path Path to the directory.
 */
import * as dntShim from "../../../../../../_dnt.shims.js";
export async function readDir(path) {
    // deno-lint-ignore no-explicit-any
    const { Deno } = dntShim.dntGlobalThis;
    path ||= ".";
    if (Deno) {
        const array = [];
        for await (const item of Deno.readDir(path)) {
            array.push(item);
        }
        return array;
    }
    const fs = await import("node:fs");
    return new Promise((resolve, reject) => {
        fs.readdir(path, (err, files) => err ? reject(err) : resolve(files.map((name) => ({ name }))));
    });
}
