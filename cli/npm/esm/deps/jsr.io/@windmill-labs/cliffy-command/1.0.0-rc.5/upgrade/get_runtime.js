/** Get runtime handler for current runtime. */
export async function getRuntime() {
    // dnt-shim-ignore deno-lint-ignore no-explicit-any
    const { Deno, process } = globalThis;
    if (Deno?.version?.deno) {
        const { DenoRuntime } = await import("./runtime/deno_runtime.js");
        return { runtimeName: "deno", runtime: new DenoRuntime() };
    }
    else if (process?.versions?.bun) {
        const { BunRuntime } = await import("./runtime/bun_runtime.js");
        return { runtimeName: "bun", runtime: new BunRuntime() };
    }
    else if (process?.versions?.node) {
        const { NodeRuntime } = await import("./runtime/node_runtime.js");
        return { runtimeName: "node", runtime: new NodeRuntime() };
    }
    else {
        throw new Error("Unsupported runtime.");
    }
}
