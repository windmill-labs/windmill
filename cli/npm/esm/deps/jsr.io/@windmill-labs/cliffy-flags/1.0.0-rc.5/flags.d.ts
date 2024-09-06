import type { FlagOptions, ParseFlagsContext, ParseFlagsOptions } from "./types.js";
/**
 * Parse command line arguments.
 *
 * @param argsOrCtx Command line arguments e.g: `Deno.args` or parse context.
 * @param opts      Parse options.
 *
 * ```
 * import { parseFlags } from "./flags.ts";
 *
 * parseFlags(Deno.args);
 * ```
 *
 * ```console
 * $ examples/flags/flags.ts -x 3 -y.z -n5 -abc --beep=boop foo bar baz --deno.land --deno.com -- --cliffy
 * {
 *   flags: {
 *     x: "3",
 *     y: { z: true },
 *     n: "5",
 *     a: true,
 *     b: true,
 *     c: true,
 *     beep: "boop",
 *     deno: { land: true, com: true }
 *   },
 *   literal: [ "--cliffy" ],
 *   unknown: [ "foo", "bar", "baz" ],
 *   stopEarly: false,
 *   stopOnUnknown: false
 * }
 * ```
 */
export declare function parseFlags<TFlags extends Record<string, unknown>, TFlagOptions extends FlagOptions, TFlagsResult extends ParseFlagsContext>(argsOrCtx: string[] | TFlagsResult, opts?: ParseFlagsOptions<TFlagOptions>): TFlagsResult & ParseFlagsContext<TFlags, TFlagOptions>;
//# sourceMappingURL=flags.d.ts.map