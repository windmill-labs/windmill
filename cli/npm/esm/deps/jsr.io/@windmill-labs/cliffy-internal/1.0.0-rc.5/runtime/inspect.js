/**
 * Inspect values.
 *
 * @internal
 */
import * as dntShim from "../../../../../../_dnt.shims.js";
export function inspect(value, colors) {
    // deno-lint-ignore no-explicit-any
    const { Deno } = dntShim.dntGlobalThis;
    return Deno?.inspect(value, { depth: 1, colors, trailingComma: false }) ?? JSON.stringify(value, null, 2);
}
