/**
 * Get environment variable.
 *
 * @internal
 * @param name The name of the environment variable.
 */
export function getEnv(name) {
    // dnt-shim-ignore deno-lint-ignore no-explicit-any
    const { Deno, process } = globalThis;
    if (Deno) {
        return Deno.env.get(name);
    }
    else if (process) {
        return process.env[name];
    }
    throw new Error("unsupported runtime");
}
