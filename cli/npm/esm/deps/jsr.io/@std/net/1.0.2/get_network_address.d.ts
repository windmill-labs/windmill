/**
 * Gets the IPv4 or IPv6 network address of the machine.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.

 *
 * This is inspired by the util of the same name in
 * {@linkcode https://www.npmjs.com/package/serve | npm:serve}.
 *
 * For more advanced use, use {@linkcode Deno.networkInterfaces} directly.
 *
 * @see {@link https://github.com/vercel/serve/blob/1ea55b1b5004f468159b54775e4fb3090fedbb2b/source/utilities/http.ts#L33}
 *
 * @param family The IP protocol version of the interface to get the address of.
 * @returns The IPv4 network address of the machine or `undefined` if not found.
 *
 * @example Get the IPv4 network address (default)
 * ```ts no-assert no-eval
 * import { getNetworkAddress } from "@std/net/get-network-address";
 *
 * const hostname = getNetworkAddress()!;
 *
 * Deno.serve({ port: 0, hostname }, () => new Response("Hello, world!"));
 * ```
 *
 * @example Get the IPv6 network address
 * ```ts no-assert no-eval
 * import { getNetworkAddress } from "@std/net/get-network-address";
 *
 * const hostname = getNetworkAddress("IPv6")!;
 *
 * Deno.serve({ port: 0, hostname }, () => new Response("Hello, world!"));
 * ```
 */
import * as dntShim from "../../../../../_dnt.shims.js";
export declare function getNetworkAddress(family?: dntShim.Deno.NetworkInterfaceInfo["family"]): string | undefined;
//# sourceMappingURL=get_network_address.d.ts.map