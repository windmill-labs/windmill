/** Parser options. */
export interface ParseFlagsOptions<
  TFlagOptions extends FlagOptions = FlagOptions,
> {
  /** An array of flag options. */
  flags?: Array<TFlagOptions>;
  /** Parser callback function for custom types. */
  parse?: TypeHandler<unknown>;
  /** Option callback function. Will be called for all parsed options. */
  option?: (option: TFlagOptions, value?: unknown) => void;
  /**
   * Enable stop early. If enabled, all arguments starting from the first non
   * option argument will be added to the unknown array.
   *
   * For example:
   *     `command --debug-level warning server --port 80`
   *
   * Will result in:
   *     - flags: `{ debugLevel: 'warning' }`
   *     - unknown: `['server', '--port', '80']`
   */
  stopEarly?: boolean;
  /** Works similar to `stopEarly`, bit stops on first unknown option or argument. */
  stopOnUnknown?: boolean;
  /**
   * Don't throw an error when no arguments are passed to the `parseFlags`
   * function.
   */
  allowEmpty?: boolean;
  /** Ignore default values for defined flags. */
  ignoreDefaults?: Record<string, unknown>;
  /** Enable/disable dotted options. Default is `true`. */
  dotted?: boolean;
}

/** Flag options. */
export interface FlagOptions extends Omit<ArgumentOptions, "optional"> {
  /** The name of the flag. */
  name: string;
  /** An array of argument options. */
  args?: Array<ArgumentOptions>;
  /** Mark argument as optional. */
  optionalValue?: boolean;
  /** Name aliases. */
  aliases?: string[];
  /** If enabled, option cannot be combined with other options. */
  standalone?: boolean;
  /** Default value. */
  default?: DefaultValue;
  /** Mark flag as required. */
  required?: boolean;
  /**
   * An array of required flags that needs to be called together with this flag.
   * If one of the flags is missing, an error will be thrown.
   */
  depends?: string[];
  /**
   * An array of conflicting flags that cannot be called together with this flag.
   * If one of the flags is used together with this flag, an error will be
   * thrown.
   */
  conflicts?: string[];
  /**
   * A callback function to map the flag value(s). The first argument if the
   * callback function is the current value, and the second argument is an array
   * of previous values when combined with the `collect` option.
   */
  value?: ValueHandler;
  /**
   * If enabled, the flag can be used multiple times and all values will be
   * collected into an array.
   */
  collect?: boolean;
  /**
   * If enabled, the argument must be assigned with an equals sign to the flag
   * `--flag=<value>`, otherwise the value for this option will be ignored.
   */
  equalsSign?: boolean;
}

/** Options for a flag argument. */
export interface ArgumentOptions {
  /** Argument type. */
  type?: ArgumentType | string;
  /** Make argument optional. */
  optional?: boolean;
  /** Make argument variadic. */
  variadic?: boolean;
  /**
   * If enabled, argument will be separated by a separator defined with the
   * `separator` option.
   */
  list?: boolean;
  /** List separator. */
  separator?: string;
}

/** Available build-in argument types. */
export type ArgumentType = "string" | "boolean" | "number" | "integer";

/** Default flag value or a callback method that returns the default value. */
export type DefaultValue<TValue = unknown> =
  | TValue
  | DefaultValueHandler<TValue>;

/** Default value callback function to lazy load the default value. */
export type DefaultValueHandler<TValue = unknown> = () => TValue;

/** A callback method for custom processing or mapping of flag values. */
// deno-lint-ignore no-explicit-any
export type ValueHandler<TValue = any, TReturn = TValue> = (
  val: TValue,
  previous?: TReturn,
) => TReturn;

/**
 * Parse result. The parse context will be returned by the `parseFlags` method
 * and can be also passed as first argument to the `parseFlags` method.
 */
export interface ParseFlagsContext<
  // deno-lint-ignore no-explicit-any
  TFlags extends Record<string, any> = Record<string, any>,
  TStandaloneOption extends FlagOptions = FlagOptions,
> {
  /** An object of parsed flags. */
  flags: TFlags;
  /** An array of unknown arguments. */
  unknown: Array<string>;
  /** An array of arguments defined after the double dash ` -- `. */
  literal: Array<string>;
  /** Matched standalone option. */
  standalone?: TStandaloneOption;
  /** Is set to `true` if `stopEarly` option is enabled. */
  stopEarly: boolean;
  /** Is set to `true` if `stopOnUnknown` option is enabled. */
  stopOnUnknown: boolean;
  /** A map of option names and default values. */
  defaults: Record<string, boolean>;
}

/** Argument parsing informations. */
export interface ArgumentValue {
  /** A lable/name which describes the kind of argument, e.g: `Option`. */
  label: string;
  /** The type of the argument. */
  type: ArgumentType | string;
  /** The name of the argument. */
  name: string;
  /** The value of the argument. */
  value: string;
}

/**
 * Parse method for custom types. Gets the raw user input passed as argument
 * and returns the parsed value.
 */
export type TypeHandler<TReturn = unknown> = (arg: ArgumentValue) => TReturn;
