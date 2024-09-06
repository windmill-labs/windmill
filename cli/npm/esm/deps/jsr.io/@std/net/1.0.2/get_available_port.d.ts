export interface GetAvailablePortOptions {
    /**
     * A port to check availability of first. If the port isn't available, fall
     * back to another port.
     *
     * Defaults to port 0, which will let the operating system choose an available
     * port.
     *
     * @default {0}
     */
    preferredPort?: number;
}
/**
 * Returns an available network port.
 *
 * > [!IMPORTANT]
 * > In most cases, this function is not needed. Do not use it for trivial uses
 * > such as when using {@linkcode Deno.serve} or {@linkcode Deno.listen}
 * > directly. Instead, set the `port` option to `0` to automatically use an
 * > available port, then get the assigned port from the function's return
 * > object (see "Recommended Usage" example).
 *
 * @param options Options for getting an available port.
 * @returns An available network port.
 *
 * @example Recommended Usage
 *
 * Bad:
 * ```ts no-eval no-assert
 * import { getAvailablePort } from "@std/net/get-available-port";
 *
 * const port = getAvailablePort();
 * Deno.serve({ port }, () => new Response("Hello, world!"));
 * ```
 *
 * Good:
 * ```ts no-eval no-assert
 * const { port } = Deno.serve({ port: 0 }, () => new Response("Hello, world!")).addr;
 * ```
 *
 * Good:
 * ```ts no-eval no-assert
 * import { getAvailablePort } from "@std/net/get-available-port";
 *
 * const command = new Deno.Command(Deno.execPath(), {
 *   args: ["test.ts", "--port", getAvailablePort().toString()],
 * });
 * // ...
 * ```
 */
export declare function getAvailablePort(options?: GetAvailablePortOptions): number;
//# sourceMappingURL=get_available_port.d.ts.map