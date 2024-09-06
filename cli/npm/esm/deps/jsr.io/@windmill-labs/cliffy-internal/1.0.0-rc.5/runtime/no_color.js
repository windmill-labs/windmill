/**
 * Checks if colors are enabled.
 *
 * @internal
 */
export function getNoColor() {
    // dnt-shim-ignore deno-lint-ignore no-explicit-any
    const { Deno, process } = globalThis;
    if (Deno) {
        return Deno.noColor;
    }
    else if (process) {
        return (process?.env.NO_COLOR === "1" || process?.env.NODE_DISABLE_COLORS === "1");
    }
    throw new Error("unsupported runtime");
}
