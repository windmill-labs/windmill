/**
 * Get file info.
 *
 * @internal
 * @param input Path to the file.
 */
import * as dntShim from "../../../../../../_dnt.shims.js";
export async function stat(input) {
    // deno-lint-ignore no-explicit-any
    const { Deno } = dntShim.dntGlobalThis;
    if (Deno) {
        return Deno.stat(input);
    }
    const { statSync } = await import("node:fs");
    const stats = statSync(input);
    return {
        get isDirectory() {
            return stats.isDirectory();
        },
    };
}
