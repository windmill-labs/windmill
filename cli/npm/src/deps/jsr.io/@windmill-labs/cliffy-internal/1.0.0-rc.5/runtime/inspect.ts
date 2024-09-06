/**
 * Inspect values.
 *
 * @internal
 */
import * as dntShim from "../../../../../../_dnt.shims.js";

export function inspect(value: unknown, colors: boolean): string {
  // deno-lint-ignore no-explicit-any
  const { Deno } = dntShim.dntGlobalThis as any;

  return Deno?.inspect(
    value,
    { depth: 1, colors, trailingComma: false },
  ) ?? JSON.stringify(value, null, 2);
}
