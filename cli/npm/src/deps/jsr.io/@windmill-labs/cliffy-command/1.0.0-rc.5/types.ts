// deno-lint-ignore-file no-explicit-any

import type {
  ArgumentOptions,
  ArgumentValue,
  DefaultValue,
  FlagOptions,
  TypeHandler,
  ValueHandler,
} from "../../cliffy-flags/1.0.0-rc.5/mod.js";
import type { MapTypes } from "./_argument_types.js";
import type { ValidationError } from "./_errors.js";
import type { Command } from "./command.js";
import type { HelpOptions } from "./help/_help_generator.js";
import type { Type } from "./type.js";
import type { Merge } from "./_type_utils.js";

export type { ArgumentValue, DefaultValue, TypeHandler };

export type TypeOrTypeHandler<TValue> = Type<TValue> | TypeHandler<TValue>;

export type Id<TValue> = TValue extends Record<string, unknown>
  ? TValue extends infer U ? { [K in keyof U]: Id<U[K]> } : never
  : TValue;

/* COMMAND TYPES */

/** Description handler. */
export type Description<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> =
  | string
  | DescriptionHandler<
    TOptions,
    TArguments,
    TGlobals,
    TParentGlobals,
    TTypes,
    TGlobalTypes,
    TParentTypes,
    TParentCommand
  >;

/** Description handler. */
export type DescriptionHandler<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> = (
  this: Command<
    TParentGlobals,
    TParentTypes,
    TOptions,
    TArguments,
    TGlobals,
    TTypes,
    TGlobalTypes,
    TParentCommand
  >,
) => string;

/** Action handler for commands and options. */
export type ActionHandler<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> = (
  this: Command<
    TParentGlobals,
    TParentTypes,
    TOptions,
    TArguments,
    TGlobals,
    TTypes,
    TGlobalTypes,
    TParentCommand
  >,
  options: CommandOptions<TOptions, TGlobals, TParentGlobals>,
  ...args: MapTypes<TArguments>
) => unknown;

export type CommandOptions<
  TOptions extends Record<string, any> | void = any,
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
> = MapTypes<Merge<TParentGlobals, Merge<TGlobals, TOptions>>>;

/** Argument details. */
export interface Argument extends ArgumentOptions {
  /** Argument name. */
  name: string;
  /** Shell completion action. */
  action: string;
  /** Arguments type. */
  type: string;
}

/** Result of `cmd.parse()` method. */
export interface CommandResult<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> {
  options: Id<Merge<Merge<TParentGlobals, TGlobals>, TOptions>>;
  args: TArguments;
  literal: string[];
  cmd: Command<
    TParentGlobals,
    TParentTypes,
    TOptions,
    TArguments,
    TGlobals,
    TTypes,
    TGlobalTypes,
    TParentCommand
  >;
}

/* OPTION TYPES */

export type OptionValueHandler<TValue = any, TReturn = TValue> = ValueHandler<
  TValue,
  TReturn
>;

type ExcludedCommandOptions =
  | "name"
  | "args"
  | "type"
  | "optionalValue"
  | "requiredValue"
  | "aliases"
  | "variadic"
  | "list"
  | "value"
  | "default";

/** Command option options. */
export interface GlobalOptionOptions<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> extends Omit<FlagOptions, ExcludedCommandOptions> {
  override?: boolean;
  hidden?: boolean;
  action?: ActionHandler<
    TOptions,
    TArguments,
    TGlobals,
    TParentGlobals,
    TTypes,
    TGlobalTypes,
    TParentTypes,
    TParentCommand
  >;
  prepend?: boolean;
  value?: OptionValueHandler;
  default?: DefaultValue;
}

export interface OptionOptions<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> extends
  GlobalOptionOptions<
    TOptions,
    TArguments,
    TGlobals,
    TParentGlobals,
    TTypes,
    TGlobalTypes,
    TParentTypes,
    TParentCommand
  > {
  global?: boolean;
}

/** Command option settings. */
export interface Option<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> extends
  OptionOptions<
    TOptions,
    TArguments,
    TGlobals,
    TParentGlobals,
    TTypes,
    TGlobalTypes,
    TParentTypes,
    TParentCommand
  >,
  Omit<FlagOptions, "value"> {
  description: string;
  flags: Array<string>;
  typeDefinition?: string;
  args: Argument[];
  groupName?: string;
}

/* ENV VARS TYPES */

export type EnvVarValueHandler<TValue = any, TReturn = TValue> = (
  val: TValue,
) => TReturn;

/** Environment variable options */
export interface GlobalEnvVarOptions {
  hidden?: boolean;
  required?: boolean;
  prefix?: string | undefined;
  value?: EnvVarValueHandler;
  exitCode?: number;
}

/** Environment variable options */
export interface EnvVarOptions extends GlobalEnvVarOptions {
  global?: boolean;
}

/** Environment variable settings. */
export interface EnvVar extends EnvVarOptions {
  name: string;
  names: string[];
  description: string;
  type: string;
  details: Argument;
}

/* TYPE TYPES */

/** Type options. */
export interface TypeOptions {
  override?: boolean;
  global?: boolean;
}

/** Type settings. */
export interface TypeDef extends TypeOptions {
  name: string;
  handler: Type<unknown> | TypeHandler<unknown>;
}

/* EXAMPLE TYPES */

/** Example settings. */
export interface Example {
  name: string;
  description: string;
}

/* COMPLETION TYPES */

/** Completion options. */
export interface CompleteOptions {
  override?: boolean;
  global?: boolean;
}

/** Completion settings. */
export interface Completion<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> extends CompleteOptions {
  name: string;
  complete: CompleteHandler<
    TOptions,
    TArguments,
    TGlobals,
    TParentGlobals,
    TTypes,
    TGlobalTypes,
    TParentTypes,
    TParentCommand
  >;
}

export type CompleteHandlerResult =
  | Array<string | number | boolean>
  | Promise<Array<string | number | boolean>>;

export type ValuesHandlerResult = Array<string | number | boolean>;

/** Type parser method. */
export type CompleteHandler<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
> = (
  cmd: Command<
    TParentGlobals,
    TParentTypes,
    TOptions,
    TArguments,
    TGlobals,
    TTypes,
    TGlobalTypes,
    TParentCommand
  >,
  parent?: Command<any>,
) => CompleteHandlerResult;

/**
 * Help callback method to print the help.
 * Invoked by the `--help` option and `help` command and the `.getHelp()` and `.showHelp()` methods.
 */
export type HelpHandler<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
  TCommand extends Command<
    TParentGlobals,
    TParentTypes,
    TOptions,
    TArguments,
    TGlobals,
    TTypes,
    TGlobalTypes,
    TParentCommand
  > = Command<
    TParentGlobals,
    TParentTypes,
    TOptions,
    TArguments,
    TGlobals,
    TTypes,
    TGlobalTypes,
    TParentCommand
  >,
> = (this: TCommand, cmd: TCommand, options: HelpOptions) => string;

/**
 * Version callback method to print the version.
 * Invoked by the `--help` option command and the `.getVersion()` and `.showHelp()` methods.
 */
export type VersionHandler<
  TOptions extends Record<string, any> | void = any,
  TArguments extends Array<unknown> = TOptions extends number ? any : [],
  TGlobals extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TParentGlobals extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TTypes extends Record<string, any> | void = TOptions extends number ? any
    : void,
  TGlobalTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentTypes extends Record<string, any> | void = TOptions extends number
    ? any
    : void,
  TParentCommand extends Command<any> | undefined = TOptions extends number
    ? any
    : undefined,
  TCommand extends Command<
    TParentGlobals,
    TParentTypes,
    TOptions,
    TArguments,
    TGlobals,
    TTypes,
    TGlobalTypes,
    TParentCommand
  > = Command<
    TParentGlobals,
    TParentTypes,
    TOptions,
    TArguments,
    TGlobals,
    TTypes,
    TGlobalTypes,
    TParentCommand
  >,
> = (this: TCommand, cmd: TCommand) => string;

export type ErrorHandler = (
  error: Error | ValidationError,
  cmd: Command,
) => unknown;
