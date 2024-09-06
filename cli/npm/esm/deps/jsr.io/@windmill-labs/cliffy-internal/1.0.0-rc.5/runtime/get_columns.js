/**
 * Returns the width of the console window.
 *
 * @internal
 */
import * as dntShim from "../../../../../../_dnt.shims.js";
export function getColumns() {
    try {
        // deno-lint-ignore no-explicit-any
        const { Deno, process } = dntShim.dntGlobalThis;
        // Catch error in none tty mode: Inappropriate ioctl for device (os error 25)
        if (Deno) {
            return Deno.consoleSize().columns ?? null;
        }
        else if (process) {
            return process.stdout.columns ?? null;
        }
    }
    catch (_error) {
        return null;
    }
    throw new Error("unsupported runtime");
}
