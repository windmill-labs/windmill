import type { ArgumentOptions, ArgumentValue, DefaultValue, FlagOptions, ParseFlagsContext, ParseFlagsOptions, TypeHandler, ValueHandler } from "./types.js";
/** @deprecated Use `ParseFlagsOptions` instead. */
export type IParseOptions<TFlagOptions extends FlagOptions = FlagOptions> = ParseFlagsOptions<TFlagOptions>;
/** @deprecated Use `FlagOptions` instead. */
export type IFlagOptions = FlagOptions;
/** @deprecated Use `ArgumentOptions` instead. */
export type IFlagArgument = ArgumentOptions;
/** @deprecated Use `DefaultValue` instead. */
export type IDefaultValue<TValue = unknown> = DefaultValue<TValue>;
/** @deprecated Use `ValueHandler` instead. */
export type IFlagValueHandler<TValue = any, TReturn = TValue> = ValueHandler<TValue, TReturn>;
/** @deprecated Use `ParseFlagsContext` instead. */
export type IFlagsResult<TFlags extends Record<string, any> = Record<string, any>, TStandaloneOption extends FlagOptions = FlagOptions> = ParseFlagsContext<TFlags, TStandaloneOption>;
/** @deprecated Use `ArgumentValue` instead. */
export type ITypeInfo = ArgumentValue;
/** @deprecated Use `TypeHandler` instead. */
export type ITypeHandler<TReturn = unknown> = TypeHandler<TReturn>;
/** @deprecated Use `ArgumentType` instead. */
export declare enum OptionType {
    STRING = "string",
    NUMBER = "number",
    INTEGER = "integer",
    BOOLEAN = "boolean"
}
//# sourceMappingURL=deprecated.d.ts.map