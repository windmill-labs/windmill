export { parseFlags } from "./flags.js";
export {
  type ArgumentOptions,
  type ArgumentType,
  type ArgumentValue,
  type DefaultValue,
  type DefaultValueHandler,
  type FlagOptions,
  type ParseFlagsContext,
  type ParseFlagsOptions,
  type TypeHandler,
  type ValueHandler,
} from "./types.js";
export { boolean } from "./types/boolean.js";
export { integer } from "./types/integer.js";
export { number } from "./types/number.js";
export { string } from "./types/string.js";
export {
  type IDefaultValue,
  type IFlagArgument,
  type IFlagOptions,
  type IFlagsResult,
  type IFlagValueHandler,
  type IParseOptions,
  type ITypeHandler,
  type ITypeInfo,
  OptionType,
} from "./deprecated.js";
export {
  InvalidTypeError,
  UnexpectedArgumentAfterVariadicArgumentError,
  UnexpectedRequiredArgumentError,
  UnknownTypeError,
  ValidationError,
} from "./_errors.js";
