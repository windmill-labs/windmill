import type { ParseFlagsContext, ParseFlagsOptions } from "./types.js";
import type { FlagOptions } from "./types.js";
/**
 * Flags post validation. Validations that are not already done by the parser.
 *
 * @param ctx     Parse context.
 * @param opts    Parse options.
 * @param options Option name mappings: propertyName -> option
 */
export declare function validateFlags<TOptions extends FlagOptions = FlagOptions>(ctx: ParseFlagsContext<Record<string, unknown>>, opts: ParseFlagsOptions<TOptions>, options?: Map<string, FlagOptions>): void;
//# sourceMappingURL=_validate_flags.d.ts.map