import type { FlagOptions } from "./types.js";
/** Convert param case string to camel case. */
export declare function paramCaseToCamelCase(str: string): string;
/**
 * Find option by flag, name or alias.
 *
 * @param flags Source options array.
 * @param name  Name of the option.
 */
export declare function getOption<O extends FlagOptions>(flags: Array<O>, name: string): O | undefined;
export declare function didYouMeanOption(option: string, options: Array<FlagOptions>): string;
export declare function didYouMeanType(type: string, types: Array<string>): string;
export declare function didYouMean(message: string, type: string, types: Array<string>): string;
export declare function getFlag(name: string): string;
export declare function matchWildCardOptions(name: string, flags: Array<FlagOptions>): FlagOptions | undefined;
export declare function getDefaultValue(option: FlagOptions): unknown;
//# sourceMappingURL=_utils.d.ts.map