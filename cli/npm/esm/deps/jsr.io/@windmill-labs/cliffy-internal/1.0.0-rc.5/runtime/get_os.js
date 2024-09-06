/**
 * Get operating system name.
 *
 * @internal
 */
export function getOs() {
    // dnt-shim-ignore deno-lint-ignore no-explicit-any
    const { Deno, process } = globalThis;
    if (Deno) {
        return Deno.build.os;
    }
    else if (process) {
        return process.platform;
    }
    else {
        throw new Error("unsupported runtime");
    }
}
