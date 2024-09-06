/**
 * Set raw mode on stdin.
 *
 * @internal
 * @param mode    Enable/disable raw mode.
 * @param cbreak  Enable/disable cbreak mode.
 */
export function setRaw(mode, { cbreak } = {}) {
    // dnt-shim-ignore deno-lint-ignore no-explicit-any
    const { Deno, process } = globalThis;
    if (Deno) {
        Deno.stdin.setRaw(mode, { cbreak });
    }
    else if (process) {
        process.stdin.setRawMode(mode);
    }
    else {
        throw new Error("unsupported runtime");
    }
}
