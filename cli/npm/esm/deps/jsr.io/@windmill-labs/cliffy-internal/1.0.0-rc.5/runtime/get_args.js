/**
 * Get script arguments.
 *
 * @internal
 */
export function getArgs() {
    // dnt-shim-ignore deno-lint-ignore no-explicit-any
    const { Deno, process } = globalThis;
    return Deno?.args ?? process?.argv.slice(2) ?? [];
}
