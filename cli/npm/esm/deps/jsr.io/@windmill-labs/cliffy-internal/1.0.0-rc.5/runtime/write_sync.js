/**
 * Write data to stdout.
 *
 * @internal
 * @param data Data to write to stdout.
 */
export function writeSync(data) {
    // dnt-shim-ignore deno-lint-ignore no-explicit-any
    const { Deno, process } = globalThis;
    if (Deno) {
        return Deno.stdout.writeSync(data);
    }
    else if (process) {
        process.stdout.write(data);
        return data.byteLength;
    }
    else {
        throw new Error("unsupported runtime");
    }
}
