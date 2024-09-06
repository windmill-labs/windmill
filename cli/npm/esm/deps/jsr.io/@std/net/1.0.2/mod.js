// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
/**
 * Network utilities.
 *
 * ```ts no-assert no-eval
 * import { getNetworkAddress, getAvailablePort } from "@std/net";
 *
 * console.log(`My network IP address is ${getNetworkAddress()}`);
 *
 * const command = new Deno.Command(Deno.execPath(), {
 *  args: ["test.ts", "--port", getAvailablePort().toString()],
 * });
 *
 * // ...
 * ```
 *
 * @module
 */
export * from "./get_available_port.js";
export * from "./get_network_address.js";
