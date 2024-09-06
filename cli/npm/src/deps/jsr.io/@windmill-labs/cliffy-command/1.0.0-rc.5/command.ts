// deno-lint-ignore-file no-explicit-any
import * as dntShim from "../../../../../_dnt.shims.js";

import {
  parseFlags,
  type ParseFlagsContext,
  UnknownTypeError,
  ValidationError as FlagsValidationError,
} from "../../cliffy-flags/1.0.0-rc.5/mod.js";
import { bold, brightBlue, red } from "../../../@std/fmt/0.225.6/colors.js";
import type {
  MapTypes,
  MapValue,
  MergeOptions,
  TypedArguments,
  TypedCommandArguments,
  TypedEnv,
  TypedOption,
  TypedType,
} from "./_argument_types.js";
import {
  CommandNotFoundError,
  DefaultCommandNotFoundError,
  DuplicateCommandAliasError,
  DuplicateCommandNameError,
  DuplicateCompletionError,
  DuplicateEnvVarError,
  DuplicateExampleError,
  DuplicateOptionNameError,
  DuplicateTypeError,
  MissingArgumentError,
  MissingArgumentsError,
  MissingCommandNameError,
  MissingRequiredEnvVarError,
  NoArgumentsAllowedError,
  TooManyArgumentsError,
  TooManyEnvVarValuesError,
  UnexpectedOptionalEnvVarValueError,
  UnexpectedVariadicEnvVarValueError,
  UnknownCommandError,
  ValidationError,
} from "./_errors.js";
import { exit } from "../../cliffy-internal/1.0.0-rc.5/runtime/exit.js";
import { getArgs } from "../../cliffy-internal/1.0.0-rc.5/runtime/get_args.js";
import { getEnv } from "../../cliffy-internal/1.0.0-rc.5/runtime/get_env.js";
import type { Merge, OneOf, ValueOf } from "./_type_utils.js";
import {
  getDescription,
  parseArgumentsDefinition,
  splitArguments,
  underscoreToCamelCase,
} from "./_utils.js";
import { HelpGenerator, type HelpOptions } from "./help/_help_generator.js";
import { Type } from "./type.js";
import type {
  ActionHandler,
  Argument,
  ArgumentValue,
  CommandResult,
  CompleteHandler,
  CompleteOptions,
  Completion,
  DefaultValue,
  Description,
  EnvVar,
  EnvVarOptions,
  EnvVarValueHandler,
  ErrorHandler,
  Example,
  GlobalEnvVarOptions,
  GlobalOptionOptions,
  HelpHandler,
  Option,
  OptionOptions,
  OptionValueHandler,
  TypeDef,
  TypeOptions,
  TypeOrTypeHandler,
  VersionHandler,
} from "./types.js";
import { BooleanType } from "./types/boolean.js";
import { FileType } from "./types/file.js";
import { IntegerType } from "./types/integer.js";
import { NumberType } from "./types/number.js";
import { StringType } from "./types/string.js";
import { checkVersion } from "./upgrade/_check_version.js";

/**
 * Chainable command factory class.
 *
 * ```ts
 * import { Command } from "./mod.ts";
 *
 * export const cli = new Command()
 *   .name("todo")
 *   .description("Example command description")
 *   .globalOption("--verbose", "Enable verbose output.")
 *   .globalEnv("VERBOSE=<value>", "Enable verbose output.")
 *   .command("add <todo>", "Add todo.")
 *   .action(({ verbose }, todo: string) => {
 *     if (verbose) {
 *       console.log("Add todo '%s'.", todo);
 *     }
 *   })
 *   .command("delete <id>", "Delete todo.")
 *   .action(({ verbose }, id: string) => {
 *     if (verbose) {
 *       console.log("Delete todo with id '%s'.", id);
 *     }
 *   });
 *
 * if (import.meta.main) {
 *   await cli.parse();
 * }
 * ```
 */
export class Command<
  TParentCommandGlobals extends Record<string, unknown> | void = void,
  TParentCommandTypes extends Record<string, unknown> | void =
    TParentCommandGlobals extends number ? any : void,
  TCommandOptions extends Record<string, unknown> | void =
    TParentCommandGlobals extends number ? any : void,
  TCommandArguments extends Array<unknown> = TParentCommandGlobals extends
    number ? any : [],
  TCommandGlobals extends Record<string, unknown> | void =
    TParentCommandGlobals extends number ? any : void,
  TCommandTypes extends Record<string, unknown> | void =
    TParentCommandGlobals extends number ? any : {
      number: number;
      integer: number;
      string: string;
      boolean: boolean;
      file: string;
    },
  TCommandGlobalTypes extends Record<string, unknown> | void =
    TParentCommandGlobals extends number ? any : void,
  TParentCommand extends Command<any> | undefined =
    TParentCommandGlobals extends number ? any : undefined,
> {
  private types: Map<string, TypeDef> = new Map();
  private rawArgs: Array<string> = [];
  private literalArgs: Array<string> = [];
  private _name = "COMMAND";
  private _parent?: TParentCommand;
  private _globalParent?: Command<any>;
  private ver?: VersionHandler;
  private desc: Description = "";
  private _usage?: string;
  private actionHandler?: ActionHandler;
  private globalActionHandler?: ActionHandler;
  private options: Array<Option> = [];
  private commands = new Map<string, Command<any>>();
  private examples: Array<Example> = [];
  private envVars: Array<EnvVar> = [];
  private aliases: Array<string> = [];
  private completions = new Map<string, Completion>();
  private cmd: Command<any> = this;
  private argsDefinition?: string;
  private throwOnError = false;
  private _allowEmpty = false;
  private _stopEarly = false;
  private defaultCommand?: string;
  private _useRawArgs = false;
  private args: Array<Argument> = [];
  private isHidden = false;
  private isGlobal = false;
  private hasDefaults = false;
  private _versionOptions?: DefaultOption | false;
  private _helpOptions?: DefaultOption | false;
  private _versionOption?: Option;
  private _helpOption?: Option;
  private _help?: HelpHandler;
  private _shouldExit?: boolean;
  private _meta: Record<string, string> = {};
  private _groupName: string | null = null;
  private _noGlobals = false;
  private errorHandler?: ErrorHandler;

  /** Disable version option. */
  public versionOption(enable: false): this;

  /**
   * Set global version option.
   *
   * @param flags The flags of the version option.
   * @param desc  The description of the version option.
   * @param opts  Version option options.
   */
  public versionOption(
    flags: string,
    desc?: string,
    opts?:
      & OptionOptions<
        Partial<TCommandOptions>,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
      & {
        global: true;
      },
  ): this;

  /**
   * Set version option.
   *
   * @param flags The flags of the version option.
   * @param desc  The description of the version option.
   * @param opts  Version option options.
   */
  public versionOption(
    flags: string,
    desc?: string,
    opts?: OptionOptions<
      TCommandOptions,
      TCommandArguments,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      TParentCommand
    >,
  ): this;

  /**
   * Set version option.
   *
   * @param flags The flags of the version option.
   * @param desc  The description of the version option.
   * @param opts  The action of the version option.
   */
  public versionOption(
    flags: string,
    desc?: string,
    opts?: ActionHandler<
      TCommandOptions,
      TCommandArguments,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      TParentCommand
    >,
  ): this;

  public versionOption(
    flags: string | false,
    desc?: string,
    opts?:
      | ActionHandler<
        TCommandOptions,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
      | OptionOptions<
        TCommandOptions,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
      | OptionOptions<
        Partial<TCommandOptions>,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
        & {
          global: true;
        },
  ): this {
    this._versionOptions = flags === false ? flags : {
      flags,
      desc,
      opts: typeof opts === "function" ? { action: opts } : opts,
    };
    return this;
  }

  /** Disable help option. */
  public helpOption(enable: false): this;

  /**
   * Set global help option.
   *
   * @param flags The flags of the help option.
   * @param desc  The description of the help option.
   * @param opts  Help option options.
   */
  public helpOption(
    flags: string,
    desc?: string,
    opts?:
      & OptionOptions<
        Partial<TCommandOptions>,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
      & {
        global: true;
      },
  ): this;

  /**
   * Set help option.
   *
   * @param flags The flags of the help option.
   * @param desc  The description of the help option.
   * @param opts  Help option options.
   */
  public helpOption(
    flags: string,
    desc?: string,
    opts?: OptionOptions<
      TCommandOptions,
      TCommandArguments,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      TParentCommand
    >,
  ): this;

  /**
   * Set help option.
   *
   * @param flags The flags of the help option.
   * @param desc  The description of the help option.
   * @param opts  The action of the help option.
   */
  public helpOption(
    flags: string,
    desc?: string,
    opts?: ActionHandler<
      TCommandOptions,
      TCommandArguments,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      TParentCommand
    >,
  ): this;

  public helpOption(
    flags: string | false,
    desc?: string,
    opts?:
      | ActionHandler<
        TCommandOptions,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
      | OptionOptions<
        TCommandOptions,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
      | OptionOptions<
        Partial<TCommandOptions>,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
        & {
          global: true;
        },
  ): this {
    this._helpOptions = flags === false ? flags : {
      flags,
      desc,
      opts: typeof opts === "function" ? { action: opts } : opts,
    };
    return this;
  }

  /**
   * Add new sub-command.
   *
   * @param name      Command definition. E.g: `my-command <input-file:string> <output-file:string>`
   * @param cmd       The new child command to register.
   * @param override  Override existing child command.
   */
  public command<
    TCommand extends Command<
      (TGlobalOptions & Record<string, unknown>) | void | undefined,
      TGlobalTypes | void | undefined,
      Record<string, unknown> | void,
      Array<unknown>,
      Record<string, unknown> | void,
      Record<string, unknown> | void,
      Record<string, unknown> | void,
      Command<
        TGlobalOptions | void | undefined,
        TGlobalTypes | void | undefined,
        Record<string, unknown> | void,
        Array<unknown>,
        Record<string, unknown> | void,
        Record<string, unknown> | void,
        Record<string, unknown> | void,
        undefined
      >
    >,
    TGlobalOptions
      extends (TParentCommand extends Command<any> ? TParentCommandGlobals
        : Merge<TParentCommandGlobals, TCommandGlobals>),
    TGlobalTypes
      extends (TParentCommand extends Command<any> ? TParentCommandTypes
        : Merge<TParentCommandTypes, TCommandTypes>),
  >(
    name: string,
    cmd: TCommand,
    override?: boolean,
  ): ReturnType<TCommand["reset"]> extends Command<
    Record<string, unknown> | void,
    Record<string, unknown> | void,
    infer Options,
    infer Arguments,
    infer GlobalOptions,
    infer Types,
    infer GlobalTypes,
    undefined
  > ? Command<
      TGlobalOptions,
      TGlobalTypes,
      Options,
      Arguments,
      GlobalOptions,
      Types,
      GlobalTypes,
      OneOf<TParentCommand, this>
    >
    : never;

  /**
   * Add new sub-command.
   *
   * @param name      Command definition. E.g: `my-command <input-file:string> <output-file:string>`
   * @param cmd       The new child command to register.
   * @param override  Override existing child command.
   */
  public command<
    TCommand extends Command<
      TGlobalOptions | void | undefined,
      TGlobalTypes | void | undefined,
      Record<string, unknown> | void,
      Array<unknown>,
      Record<string, unknown> | void,
      Record<string, unknown> | void,
      Record<string, unknown> | void,
      OneOf<TParentCommand, this> | undefined
    >,
    TGlobalOptions
      extends (TParentCommand extends Command<any> ? TParentCommandGlobals
        : Merge<TParentCommandGlobals, TCommandGlobals>),
    TGlobalTypes
      extends (TParentCommand extends Command<any> ? TParentCommandTypes
        : Merge<TParentCommandTypes, TCommandTypes>),
  >(
    name: string,
    cmd: TCommand,
    override?: boolean,
  ): TCommand extends Command<
    Record<string, unknown> | void,
    Record<string, unknown> | void,
    infer Options,
    infer Arguments,
    infer GlobalOptions,
    infer Types,
    infer GlobalTypes,
    OneOf<TParentCommand, this> | undefined
  > ? Command<
      TGlobalOptions,
      TGlobalTypes,
      Options,
      Arguments,
      GlobalOptions,
      Types,
      GlobalTypes,
      OneOf<TParentCommand, this>
    >
    : never;

  /**
   * Add new sub-command.
   *
   * @param nameAndArguments  Command definition. E.g: `my-command <input-file:string> <output-file:string>`
   * @param desc              The description of the new child command.
   * @param override          Override existing child command.
   */
  public command<
    TNameAndArguments extends string,
    TArguments extends TypedCommandArguments<
      TNameAndArguments,
      TParentCommand extends Command<any> ? TParentCommandTypes
        : Merge<TParentCommandTypes, TCommandGlobalTypes>
    >,
  >(
    nameAndArguments: TNameAndArguments,
    desc?: string,
    override?: boolean,
  ): TParentCommandGlobals extends number ? Command<any> : Command<
    TParentCommand extends Command<any> ? TParentCommandGlobals
      : Merge<TParentCommandGlobals, TCommandGlobals>,
    TParentCommand extends Command<any> ? TParentCommandTypes
      : Merge<TParentCommandTypes, TCommandGlobalTypes>,
    void,
    TArguments,
    void,
    void,
    void,
    OneOf<TParentCommand, this>
  >;

  /**
   * Add new sub-command.
   * @param nameAndArguments  Command definition. E.g: `my-command <input-file:string> <output-file:string>`
   * @param cmdOrDescription  The description of the new child command.
   * @param override          Override existing child command.
   */
  command(
    nameAndArguments: string,
    cmdOrDescription?: Command<any> | string,
    override?: boolean,
  ): Command<any> {
    this.reset();

    const result = splitArguments(nameAndArguments);

    const name: string | undefined = result.flags.shift();
    const aliases: string[] = result.flags;

    if (!name) {
      throw new MissingCommandNameError();
    }

    if (this.getBaseCommand(name, true)) {
      if (!override) {
        throw new DuplicateCommandNameError(name);
      }
      this.removeCommand(name);
    }

    let description: string | undefined;
    let cmd: Command<any>;

    if (typeof cmdOrDescription === "string") {
      description = cmdOrDescription;
    }

    if (cmdOrDescription instanceof Command) {
      cmd = cmdOrDescription.reset();
    } else {
      cmd = new Command();
    }

    cmd._name = name;
    cmd._parent = this;

    if (description) {
      cmd.description(description);
    }

    if (result.typeDefinition) {
      cmd.arguments(result.typeDefinition);
    }

    aliases.forEach((alias: string) => cmd.alias(alias));

    this.commands.set(name, cmd);

    this.select(name);

    return this;
  }

  /**
   * Add new command alias.
   *
   * @param alias Tha name of the alias.
   */
  public alias(alias: string): this {
    if (this.cmd._name === alias || this.cmd.aliases.includes(alias)) {
      throw new DuplicateCommandAliasError(alias);
    }

    this.cmd.aliases.push(alias);

    return this;
  }

  /** Reset internal command reference to main command. */
  public reset(): OneOf<TParentCommand, this> {
    this._groupName = null;
    this.cmd = this;
    return this as OneOf<TParentCommand, this>;
  }

  /**
   * Set internal command pointer to child command with given name.
   * @param name The name of the command to select.
   */
  public select<
    TOptions extends Record<string, unknown> | void = any,
    TArguments extends Array<unknown> = any,
    TGlobalOptions extends Record<string, unknown> | void = any,
  >(
    name: string,
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    TOptions,
    TArguments,
    TGlobalOptions,
    TCommandTypes,
    TCommandGlobalTypes,
    TParentCommand
  > {
    const cmd = this.getBaseCommand(name, true);

    if (!cmd) {
      throw new CommandNotFoundError(name, this.getBaseCommands(true));
    }

    this.cmd = cmd;

    return this as Command<any>;
  }

  /*****************************************************************************
   **** SUB HANDLER ************************************************************
   *****************************************************************************/

  /** Set command name. Used in auto generated help and shell completions */
  public name(name: string): this {
    this.cmd._name = name;
    return this;
  }

  /**
   * Set command version.
   *
   * @param version Semantic version string string or method that returns the version string.
   */
  public version(
    version:
      | string
      | VersionHandler<
        Partial<TCommandOptions>,
        Partial<TCommandArguments>,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >,
  ): this {
    if (typeof version === "string") {
      this.cmd.ver = () => version;
    } else if (typeof version === "function") {
      this.cmd.ver = version;
    }
    return this;
  }

  /**
   * Add meta data. Will be displayed in the auto generated help and in the
   * output of the long version.
   *
   * @param name  The name/label of the metadata.
   * @param value The value of the metadata.
   */
  public meta(name: string, value: string): this {
    this.cmd._meta[name] = value;
    return this;
  }

  /** Returns an object of metadata. */
  public getMeta(): Record<string, string>;

  /** Get metadata value by name. */
  public getMeta(name: string): string;

  public getMeta(name?: string): Record<string, string> | string {
    return typeof name === "undefined" ? this._meta : this._meta[name];
  }

  /**
   * Set command help.
   *
   * @param help Help string, method, or config for generator that returns the help string.
   */
  public help(
    help:
      | string
      | HelpHandler<
        Partial<TCommandOptions>,
        Partial<TCommandArguments>,
        TCommandGlobals,
        TParentCommandGlobals
      >
      | HelpOptions,
  ): this {
    if (typeof help === "string") {
      this.cmd._help = () => help;
    } else if (typeof help === "function") {
      this.cmd._help = help;
    } else {
      this.cmd._help = (cmd: Command, options: HelpOptions): string =>
        HelpGenerator.generate(cmd, { ...help, ...options });
    }
    return this;
  }

  /**
   * Set the long command description.
   *
   * @param description The command description.
   */
  public description(
    description: Description<
      TCommandOptions,
      TCommandArguments,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      TParentCommand
    >,
  ): this {
    this.cmd.desc = description;
    return this;
  }

  /**
   * Set the command usage. Defaults to arguments.
   *
   * @param usage The command usage.
   */
  public usage(usage: string): this {
    this.cmd._usage = usage;
    return this;
  }

  /** Hide command from help, completions, etc. */
  public hidden(): this {
    this.cmd.isHidden = true;
    return this;
  }

  /** Make command globally available. */
  public global(): this {
    this.cmd.isGlobal = true;
    return this;
  }

  /**
   * Set command arguments.
   *
   * Syntax: `<requiredArg:string> [optionalArg: number] [...restArgs:string]`
   */
  public arguments<
    TArguments extends TypedArguments<
      TArgs,
      Merge<TParentCommandTypes, Merge<TCommandGlobalTypes, TCommandTypes>>
    >,
    TArgs extends string = string,
  >(
    args: TArgs,
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    TCommandOptions,
    TArguments,
    TCommandGlobals,
    TCommandTypes,
    TCommandGlobalTypes,
    TParentCommand
  > {
    this.cmd.argsDefinition = args;
    return this as Command<any>;
  }

  /**
   * Set command callback method.
   *
   * @param fn Command action handler.
   */
  public action(
    fn: ActionHandler<
      TCommandOptions,
      TCommandArguments,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      TParentCommand
    >,
  ): this {
    this.cmd.actionHandler = fn;
    return this;
  }

  /**
   * Set command callback method.
   *
   * @param fn Command action handler.
   */
  public globalAction(
    fn: ActionHandler<
      Partial<TCommandOptions>,
      Array<unknown>,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      TParentCommand
    >,
  ): this {
    this.cmd.globalActionHandler = fn;
    return this;
  }

  /**
   * Don't throw an error if the command was called without arguments.
   *
   * @param allowEmpty Enable/disable allow empty.
   */
  public allowEmpty<TAllowEmpty extends boolean | undefined = undefined>(
    allowEmpty?: TAllowEmpty,
  ): false extends TAllowEmpty ? this
    : Command<
      Partial<TParentCommandGlobals>,
      TParentCommandTypes,
      Partial<TCommandOptions>,
      TCommandArguments,
      TCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommand
    > {
    this.cmd._allowEmpty = allowEmpty !== false;
    return this as false extends TAllowEmpty ? this
      : Command<
        Partial<TParentCommandGlobals>,
        TParentCommandTypes,
        Partial<TCommandOptions>,
        TCommandArguments,
        TCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommand
      >;
  }

  /**
   * Enable stop early. If enabled, all arguments starting from the first non
   * option argument will be passed as arguments with type string to the command
   * action handler.
   *
   * For example:
   *     `command --debug-level warning server --port 80`
   *
   * Will result in:
   *     - options: `{ debugLevel: 'warning' }`
   *     - args: `['server', '--port', '80']`
   *
   * @param stopEarly Enable/disable stop early.
   */
  public stopEarly(stopEarly = true): this {
    this.cmd._stopEarly = stopEarly;
    return this;
  }

  /**
   * Disable parsing arguments. If enabled the raw arguments will be passed to
   * the action handler. This has no effect for parent or child commands. Only
   * for the command on which this method was called.
   *
   * @param useRawArgs Enable/disable raw arguments.
   */
  public useRawArgs(
    useRawArgs = true,
  ): Command<
    void,
    void,
    void,
    Array<string>,
    void,
    void,
    void,
    TParentCommand
  > {
    this.cmd._useRawArgs = useRawArgs;
    return this as Command<any>;
  }

  /**
   * Set default command. The default command is executed when the program
   * was called without any argument and if no action handler is registered.
   *
   * @param name Name of the default command.
   */
  public default(name: string): this {
    this.cmd.defaultCommand = name;
    return this;
  }

  public globalType<
    THandler extends TypeOrTypeHandler<unknown>,
    TName extends string = string,
  >(
    name: TName,
    handler: THandler,
    options?: Omit<TypeOptions, "global">,
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    TCommandOptions,
    TCommandArguments,
    TCommandGlobals,
    TCommandTypes,
    Merge<TCommandGlobalTypes, TypedType<TName, THandler>>,
    TParentCommand
  > {
    return this.type(name, handler, { ...options, global: true });
  }

  /**
   * Register custom type.
   *
   * @param name    The name of the type.
   * @param handler The callback method to parse the type.
   * @param options Type options.
   */
  public type<
    THandler extends TypeOrTypeHandler<unknown>,
    TName extends string = string,
  >(
    name: TName,
    handler: THandler,
    options?: TypeOptions,
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    TCommandOptions,
    TCommandArguments,
    TCommandGlobals,
    Merge<TCommandTypes, TypedType<TName, THandler>>,
    TCommandGlobalTypes,
    TParentCommand
  > {
    if (this.cmd.types.get(name) && !options?.override) {
      throw new DuplicateTypeError(name);
    }

    this.cmd.types.set(name, {
      ...options,
      name,
      handler: handler as TypeOrTypeHandler<unknown>,
    });

    if (
      handler instanceof Type &&
      (typeof handler.complete !== "undefined" ||
        typeof handler.values !== "undefined")
    ) {
      const completeHandler: CompleteHandler = (
        cmd: Command,
        parent?: Command,
      ) => handler.complete?.(cmd, parent) || [];
      this.complete(name, completeHandler, options);
    }

    return this as Command<any>;
  }

  /**
   * Register global complete handler.
   *
   * @param name      The name of the completion.
   * @param complete  The callback method to complete the type.
   * @param options   Complete options.
   */
  public globalComplete(
    name: string,
    complete: CompleteHandler,
    options?: Omit<CompleteOptions, "global">,
  ): this {
    return this.complete(name, complete, { ...options, global: true });
  }

  /**
   * Register global complete handler.
   *
   * @param name      The name of the completion.
   * @param complete  The callback method to complete the type.
   * @param options   Complete options.
   */
  public complete(
    name: string,
    complete: CompleteHandler<
      Partial<TCommandOptions>,
      Partial<TCommandArguments>,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      any
    >,
    options: CompleteOptions & { global: boolean },
  ): this;

  /**
   * Register complete handler.
   *
   * @param name      The name of the completion.
   * @param complete  The callback method to complete the type.
   * @param options   Complete options.
   */
  public complete(
    name: string,
    complete: CompleteHandler<
      TCommandOptions,
      TCommandArguments,
      TCommandGlobals,
      TParentCommandGlobals,
      TCommandTypes,
      TCommandGlobalTypes,
      TParentCommandTypes,
      TParentCommand
    >,
    options?: CompleteOptions,
  ): this;

  public complete(
    name: string,
    complete:
      | CompleteHandler<
        TCommandOptions,
        TCommandArguments,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
      | CompleteHandler<
        Partial<TCommandOptions>,
        Partial<TCommandArguments>,
        TCommandGlobals,
        TParentCommandGlobals,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        any
      >,
    options?: CompleteOptions,
  ): this {
    if (this.cmd.completions.has(name) && !options?.override) {
      throw new DuplicateCompletionError(name);
    }

    this.cmd.completions.set(name, {
      name,
      complete,
      ...options,
    });

    return this;
  }

  /**
   * Throw validation errors instead of calling `exit()` to handle
   * validation errors manually.
   *
   * A validation error is thrown when the command is wrongly used by the user.
   * For example: If the user passes some invalid options or arguments to the
   * command.
   *
   * This has no effect for parent commands. Only for the command on which this
   * method was called and all child commands.
   *
   * **Example:**
   *
   * ```ts
   * import { Command, ValidationError } from "./mod.ts";
   *
   * const cmd = new Command();
   * // ...
   *
   * try {
   *   cmd.parse();
   * } catch(error) {
   *   if (error instanceof ValidationError) {
   *     cmd.showHelp();
   *     Deno.exit(1);
   *   }
   *   throw error;
   * }
   * ```
   *
   * @see ValidationError
   */
  public throwErrors(): this {
    this.cmd.throwOnError = true;
    return this;
  }

  /**
   * Set custom error handler.
   *
   * @param handler Error handler callback function.
   */
  public error(handler: ErrorHandler): this {
    this.cmd.errorHandler = handler;
    return this;
  }

  /** Get error handler callback function. */
  private getErrorHandler(): ErrorHandler | undefined {
    return this.errorHandler ?? this._parent?.errorHandler;
  }

  /**
   * Same as `.throwErrors()` but also prevents calling `exit()` after
   * printing help or version with the --help and --version option.
   */
  public noExit(): this {
    this.cmd._shouldExit = false;
    this.throwErrors();
    return this;
  }

  /**
   * Disable inheriting global commands, options and environment variables from
   * parent commands.
   */
  public noGlobals(): this {
    this.cmd._noGlobals = true;
    return this;
  }

  /** Check whether the command should throw errors or exit. */
  protected shouldThrowErrors(): boolean {
    return this.throwOnError || !!this._parent?.shouldThrowErrors();
  }

  /** Check whether the command should exit after printing help or version. */
  protected shouldExit(): boolean {
    return this._shouldExit ?? this._parent?.shouldExit() ?? true;
  }

  /**
   * Enable grouping of options and set the name of the group.
   * All option which are added after calling the `.group()` method will be
   * grouped in the help output. If the `.group()` method can be use multiple
   * times to create more groups.
   *
   * @param name The name of the option group.
   */
  public group(name: string | null): this {
    this.cmd._groupName = name;
    return this;
  }

  /**
   * Register a global option.
   *
   * @param flags Flags string e.g: -h, --help, --manual <requiredArg:string> [optionalArg:number] [...restArgs:string]
   * @param desc Flag description.
   * @param opts Flag options or custom handler for processing flag value.
   */
  public globalOption<
    TFlags extends string,
    TGlobalOptions extends TypedOption<
      TFlags,
      TCommandOptions,
      Merge<TParentCommandTypes, Merge<TCommandGlobalTypes, TCommandTypes>>,
      undefined extends TConflicts ? TRequired : false,
      TDefaultValue
    >,
    TMappedGlobalOptions extends MapValue<
      TGlobalOptions,
      TMappedValue,
      TCollect
    >,
    TRequired extends OptionOptions["required"] = undefined,
    TCollect extends OptionOptions["collect"] = undefined,
    TConflicts extends OptionOptions["conflicts"] = undefined,
    const TDefaultValue = undefined,
    TMappedValue = undefined,
  >(
    flags: TFlags,
    desc: string,
    opts?:
      | Omit<
        GlobalOptionOptions<
          Partial<TCommandOptions>,
          TCommandArguments,
          MergeOptions<TFlags, TCommandGlobals, TGlobalOptions>,
          TParentCommandGlobals,
          TCommandTypes,
          TCommandGlobalTypes,
          TParentCommandTypes,
          TParentCommand
        >,
        "value"
      >
        & {
          default?: DefaultValue<TDefaultValue>;
          required?: TRequired;
          collect?: TCollect;
          value?: OptionValueHandler<
            MapTypes<ValueOf<TGlobalOptions>>,
            TMappedValue
          >;
        }
      | OptionValueHandler<MapTypes<ValueOf<TGlobalOptions>>, TMappedValue>,
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    TCommandOptions,
    TCommandArguments,
    MergeOptions<TFlags, TCommandGlobals, TMappedGlobalOptions>,
    TCommandTypes,
    TCommandGlobalTypes,
    TParentCommand
  > {
    if (typeof opts === "function") {
      return this.option(
        flags,
        desc,
        { value: opts, global: true } as OptionOptions,
      ) as Command<any>;
    }
    return this.option(
      flags,
      desc,
      { ...opts, global: true } as OptionOptions,
    ) as Command<any>;
  }

  /**
   * Add a global option.
   *
   * @param flags Flags string e.g: -h, --help, --manual <requiredArg:string> [optionalArg:number] [...restArgs:string]
   * @param desc Flag description.
   * @param opts Flag options or custom handler for processing flag value.
   */
  public option<
    TFlags extends string,
    TGlobalOptions extends TypedOption<
      TFlags,
      TCommandOptions,
      Merge<TParentCommandTypes, Merge<TCommandGlobalTypes, TCommandTypes>>,
      undefined extends TConflicts ? TRequired : false,
      TDefaultValue
    >,
    TMappedGlobalOptions extends MapValue<
      TGlobalOptions,
      TMappedValue,
      TCollect
    >,
    TRequired extends OptionOptions["required"] = undefined,
    TCollect extends OptionOptions["collect"] = undefined,
    TConflicts extends OptionOptions["conflicts"] = undefined,
    const TDefaultValue = undefined,
    TMappedValue = undefined,
  >(
    flags: TFlags,
    desc: string,
    opts:
      | Omit<
        OptionOptions<
          Partial<TCommandOptions>,
          TCommandArguments,
          MergeOptions<TFlags, TCommandGlobals, TGlobalOptions>,
          TParentCommandGlobals,
          TCommandTypes,
          TCommandGlobalTypes,
          TParentCommandTypes,
          TParentCommand
        >,
        "value"
      >
        & {
          global: true;
          default?: DefaultValue<TDefaultValue>;
          required?: TRequired;
          collect?: TCollect;
          value?: OptionValueHandler<
            MapTypes<ValueOf<TGlobalOptions>>,
            TMappedValue
          >;
        }
      | OptionValueHandler<MapTypes<ValueOf<TGlobalOptions>>, TMappedValue>,
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    TCommandOptions,
    TCommandArguments,
    MergeOptions<TFlags, TCommandGlobals, TMappedGlobalOptions>,
    TCommandTypes,
    TCommandGlobalTypes,
    TParentCommand
  >;

  /**
   * Register an option.
   *
   * @param flags Flags string e.g: -h, --help, --manual <requiredArg:string> [optionalArg:number] [...restArgs:string]
   * @param desc Flag description.
   * @param opts Flag options or custom handler for processing flag value.
   */
  public option<
    TFlags extends string,
    TOptions extends TypedOption<
      TFlags,
      TCommandOptions,
      Merge<TParentCommandTypes, Merge<TCommandGlobalTypes, TCommandTypes>>,
      undefined extends TConflicts ? TRequired : false,
      TDefaultValue
    >,
    TMappedOptions extends MapValue<TOptions, TMappedValue, TCollect>,
    TRequired extends OptionOptions["required"] = undefined,
    TCollect extends OptionOptions["collect"] = undefined,
    TConflicts extends OptionOptions["conflicts"] = undefined,
    const TDefaultValue = undefined,
    TMappedValue = undefined,
  >(
    flags: TFlags,
    desc: string,
    opts?:
      | Omit<
        OptionOptions<
          MergeOptions<TFlags, TCommandOptions, TMappedOptions>,
          TCommandArguments,
          TCommandGlobals,
          TParentCommandGlobals,
          TCommandTypes,
          TCommandGlobalTypes,
          TParentCommandTypes,
          TParentCommand
        >,
        "value"
      >
        & {
          default?: DefaultValue<TDefaultValue>;
          required?: TRequired;
          collect?: TCollect;
          conflicts?: TConflicts;
          value?: OptionValueHandler<MapTypes<ValueOf<TOptions>>, TMappedValue>;
        }
      | OptionValueHandler<MapTypes<ValueOf<TOptions>>, TMappedValue>,
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    MergeOptions<TFlags, TCommandOptions, TMappedOptions>,
    TCommandArguments,
    TCommandGlobals,
    TCommandTypes,
    TCommandGlobalTypes,
    TParentCommand
  >;

  public option(
    flags: string,
    desc: string,
    opts?: OptionOptions | OptionValueHandler,
  ): Command<any> {
    if (typeof opts === "function") {
      opts = { value: opts };
    }

    const result = splitArguments(flags);

    const args: Argument[] = result.typeDefinition
      ? parseArgumentsDefinition(result.typeDefinition)
      : [];

    const option: Option = {
      ...opts,
      name: "",
      description: desc,
      args,
      flags: result.flags,
      equalsSign: result.equalsSign,
      typeDefinition: result.typeDefinition,
      groupName: this._groupName ?? undefined,
    };

    if (option.separator) {
      for (const arg of args) {
        if (arg.list) {
          arg.separator = option.separator;
        }
      }
    }

    for (const part of option.flags) {
      const arg = part.trim();
      const isLong = /^--/.test(arg);
      const name = isLong ? arg.slice(2) : arg.slice(1);

      if (this.cmd.getBaseOption(name, true)) {
        if (opts?.override) {
          this.removeOption(name);
        } else {
          throw new DuplicateOptionNameError(name, this.getPath());
        }
      }

      if (!option.name && isLong) {
        option.name = name;
      } else if (!option.aliases) {
        option.aliases = [name];
      } else {
        option.aliases.push(name);
      }
    }

    if (option.prepend) {
      this.cmd.options.unshift(option);
    } else {
      this.cmd.options.push(option);
    }

    return this;
  }

  /**
   * Register command example.
   *
   * @param name          Name of the example.
   * @param description   The content of the example.
   */
  public example(name: string, description: string): this {
    if (this.cmd.hasExample(name)) {
      throw new DuplicateExampleError(name);
    }

    this.cmd.examples.push({ name, description });

    return this;
  }

  /**
   * @param flags Flags string e.g: -h, --help, --manual <requiredArg:string> [optionalArg:number] [...restArgs:string]
   * @param desc Flag description.
   * @param opts Flag options or custom handler for processing flag value.
   */

  /**
   * Register a global environment variable.
   *
   * @param name        Name of the environment variable.
   * @param description The description of the environment variable.
   * @param options     Environment variable options.
   */
  public globalEnv<
    TNameAndValue extends string,
    TGlobalEnvVars extends TypedEnv<
      TNameAndValue,
      TPrefix,
      TCommandOptions,
      Merge<TParentCommandTypes, Merge<TCommandGlobalTypes, TCommandTypes>>,
      TRequired
    >,
    TMappedGlobalEnvVars extends MapValue<TGlobalEnvVars, TMappedValue>,
    TRequired extends EnvVarOptions["required"] = undefined,
    TPrefix extends EnvVarOptions["prefix"] = undefined,
    TMappedValue = undefined,
  >(
    name: TNameAndValue,
    description: string,
    options?: Omit<GlobalEnvVarOptions, "value"> & {
      required?: TRequired;
      prefix?: TPrefix;
      value?: EnvVarValueHandler<
        MapTypes<ValueOf<TGlobalEnvVars>>,
        TMappedValue
      >;
    },
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    TCommandOptions,
    TCommandArguments,
    Merge<TCommandGlobals, TMappedGlobalEnvVars>,
    TCommandTypes,
    TCommandGlobalTypes,
    TParentCommand
  > {
    return this.env(
      name,
      description,
      { ...options, global: true } as EnvVarOptions,
    ) as Command<any>;
  }

  /**
   * Register a global environment variable.
   *
   * @param name        Name of the environment variable.
   * @param description The description of the environment variable.
   * @param options     Environment variable options.
   */
  public env<
    N extends string,
    G extends TypedEnv<
      N,
      P,
      TCommandOptions,
      Merge<TParentCommandTypes, Merge<TCommandGlobalTypes, TCommandTypes>>,
      R
    >,
    MG extends MapValue<G, V>,
    R extends EnvVarOptions["required"] = undefined,
    P extends EnvVarOptions["prefix"] = undefined,
    V = undefined,
  >(
    name: N,
    description: string,
    options: Omit<EnvVarOptions, "value"> & {
      global: true;
      required?: R;
      prefix?: P;
      value?: EnvVarValueHandler<MapTypes<ValueOf<G>>, V>;
    },
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    TCommandOptions,
    TCommandArguments,
    Merge<TCommandGlobals, MG>,
    TCommandTypes,
    TCommandGlobalTypes,
    TParentCommand
  >;

  /**
   * Register an environment variable.
   *
   * @param name        Name of the environment variable.
   * @param description The description of the environment variable.
   * @param options     Environment variable options.
   */
  public env<
    TNameAndValue extends string,
    TEnvVar extends TypedEnv<
      TNameAndValue,
      TPrefix,
      TCommandOptions,
      Merge<TParentCommandTypes, Merge<TCommandGlobalTypes, TCommandTypes>>,
      TRequired
    >,
    TMappedEnvVar extends MapValue<TEnvVar, TMappedValue>,
    TRequired extends EnvVarOptions["required"] = undefined,
    TPrefix extends EnvVarOptions["prefix"] = undefined,
    TMappedValue = undefined,
  >(
    name: TNameAndValue,
    description: string,
    options?: Omit<EnvVarOptions, "value"> & {
      required?: TRequired;
      prefix?: TPrefix;
      value?: EnvVarValueHandler<MapTypes<ValueOf<TEnvVar>>, TMappedValue>;
    },
  ): Command<
    TParentCommandGlobals,
    TParentCommandTypes,
    Merge<TCommandOptions, TMappedEnvVar>,
    TCommandArguments,
    TCommandGlobals,
    TCommandTypes,
    TCommandGlobalTypes,
    TParentCommand
  >;

  public env(
    name: string,
    description: string,
    options?: EnvVarOptions,
  ): Command<any> {
    const result = splitArguments(name);

    if (!result.typeDefinition) {
      result.typeDefinition = "<value:boolean>";
    }

    if (result.flags.some((envName) => this.cmd.getBaseEnvVar(envName, true))) {
      throw new DuplicateEnvVarError(name);
    }

    const details: Argument[] = parseArgumentsDefinition(
      result.typeDefinition,
    );

    if (details.length > 1) {
      throw new TooManyEnvVarValuesError(name);
    } else if (details.length && details[0].optional) {
      throw new UnexpectedOptionalEnvVarValueError(name);
    } else if (details.length && details[0].variadic) {
      throw new UnexpectedVariadicEnvVarValueError(name);
    }

    this.cmd.envVars.push({
      name: result.flags[0],
      names: result.flags,
      description,
      type: details[0].type,
      details: details.shift() as Argument,
      ...options,
    });

    return this;
  }

  /*****************************************************************************
   **** MAIN HANDLER ***********************************************************
   *****************************************************************************/

  /**
   * Parse command line arguments and execute matched command.
   *
   * @param args Command line args to parse. Ex: `cmd.parse( Deno.args )`
   */
  public parse(
    args: string[] = getArgs(),
  ): Promise<
    TParentCommand extends Command<any> ? CommandResult<
        Record<string, unknown>,
        Array<unknown>,
        Record<string, unknown>,
        Record<string, unknown>,
        Record<string, unknown>,
        Record<string, unknown>,
        Record<string, unknown>,
        undefined
      >
      : CommandResult<
        MapTypes<TCommandOptions>,
        MapTypes<TCommandArguments>,
        MapTypes<TCommandGlobals>,
        MapTypes<TParentCommandGlobals>,
        TCommandTypes,
        TCommandGlobalTypes,
        TParentCommandTypes,
        TParentCommand
      >
  > {
    const ctx: ParseContext = {
      unknown: args.slice(),
      flags: {},
      env: {},
      literal: [],
      stopEarly: false,
      stopOnUnknown: false,
      defaults: {},
      actions: [],
    };
    return this.parseCommand(ctx) as any;
  }

  private async parseCommand(ctx: ParseContext): Promise<CommandResult> {
    try {
      this.reset();
      this.registerDefaults();
      this.rawArgs = ctx.unknown.slice();

      if (this._useRawArgs) {
        await this.parseEnvVars(ctx, this.envVars);
        return await this.execute(ctx.env, ctx.unknown);
      }

      let preParseGlobals = false;
      let subCommand: Command<any> | undefined;

      // Pre parse globals to support: cmd --global-option sub-command --option
      if (ctx.unknown.length > 0) {
        // Detect sub command.
        subCommand = this.getSubCommand(ctx);

        if (!subCommand) {
          // Only pre parse globals if first arg ist a global option.
          const optionName = ctx.unknown[0].replace(/^-+/, "");
          const option = this.getOption(optionName, true);

          if (option?.global) {
            preParseGlobals = true;
            await this.parseGlobalOptionsAndEnvVars(ctx);
          }
        }
      }

      if (subCommand || ctx.unknown.length > 0) {
        subCommand ??= this.getSubCommand(ctx);

        if (subCommand) {
          subCommand._globalParent = this;
          return subCommand.parseCommand(ctx);
        }
      }

      // Parse rest options & env vars.
      await this.parseOptionsAndEnvVars(ctx, preParseGlobals);
      const options = { ...ctx.env, ...ctx.flags };
      const args = this.parseArguments(ctx, options);
      this.literalArgs = ctx.literal;

      // Execute option action.
      if (ctx.actions.length) {
        await Promise.all(
          ctx.actions.map((action) => action.call(this, options, ...args)),
        );

        if (ctx.standalone) {
          return {
            options,
            args,
            cmd: this,
            literal: this.literalArgs,
          };
        }
      }

      return await this.execute(options, args);
    } catch (error: unknown) {
      this.handleError(error);
    }
  }

  private getSubCommand(ctx: ParseContext) {
    const subCommand = this.getCommand(ctx.unknown[0], true);

    if (subCommand) {
      ctx.unknown.shift();
    }

    return subCommand;
  }

  private async parseGlobalOptionsAndEnvVars(
    ctx: ParseContext,
  ): Promise<void> {
    const isHelpOption = this.getHelpOption()?.flags.includes(ctx.unknown[0]);

    // Parse global env vars.
    const envVars = [
      ...this.envVars.filter((envVar) => envVar.global),
      ...this.getGlobalEnvVars(true),
    ];

    await this.parseEnvVars(ctx, envVars, !isHelpOption);

    // Parse global options.
    const options = [
      ...this.options.filter((option) => option.global),
      ...this.getGlobalOptions(true),
    ];

    this.parseOptions(ctx, options, {
      stopEarly: true,
      stopOnUnknown: true,
      dotted: false,
    });
  }

  private async parseOptionsAndEnvVars(
    ctx: ParseContext,
    preParseGlobals: boolean,
  ): Promise<void> {
    const helpOption = this.getHelpOption();
    const isVersionOption = this._versionOption?.flags.includes(ctx.unknown[0]);
    const isHelpOption = helpOption && ctx.flags?.[helpOption.name] === true;

    // Parse env vars.
    const envVars = preParseGlobals
      ? this.envVars.filter((envVar) => !envVar.global)
      : this.getEnvVars(true);

    await this.parseEnvVars(
      ctx,
      envVars,
      !isHelpOption && !isVersionOption,
    );

    // Parse options.
    const options = this.getOptions(true);

    this.parseOptions(ctx, options);
  }

  /** Register default options like `--version` and `--help`. */
  private registerDefaults(): this {
    if (this.hasDefaults || this.getParent()) {
      return this;
    }
    this.hasDefaults = true;

    this.reset();

    !this.types.has("string") &&
      this.type("string", new StringType(), { global: true });
    !this.types.has("number") &&
      this.type("number", new NumberType(), { global: true });
    !this.types.has("integer") &&
      this.type("integer", new IntegerType(), { global: true });
    !this.types.has("boolean") &&
      this.type("boolean", new BooleanType(), { global: true });
    !this.types.has("file") &&
      this.type("file", new FileType(), { global: true });

    if (!this._help) {
      this.help({});
    }

    if (this._versionOptions !== false && (this._versionOptions || this.ver)) {
      this.option(
        this._versionOptions?.flags || "-V, --version",
        this._versionOptions?.desc ||
          "Show the version number for this program.",
        {
          standalone: true,
          prepend: true,
          action: async function () {
            const long = this.getRawArgs().includes(
              `--${this._versionOption?.name}`,
            );
            if (long) {
              await checkVersion(this);
              this.showLongVersion();
            } else {
              this.showVersion();
            }
            this.exit();
          },
          ...(this._versionOptions?.opts ?? {}),
        },
      );
      this._versionOption = this.options[0];
    }

    if (this._helpOptions !== false) {
      this.option(
        this._helpOptions?.flags || "-h, --help",
        this._helpOptions?.desc || "Show this help.",
        {
          standalone: true,
          global: true,
          prepend: true,
          action: async function () {
            const long = this.getRawArgs().includes(
              `--${this.getHelpOption()?.name}`,
            );
            await checkVersion(this);
            this.showHelp({ long });
            this.exit();
          },
          ...(this._helpOptions?.opts ?? {}),
        },
      );
      this._helpOption = this.options[0];
    }

    return this;
  }

  /**
   * Execute command.
   * @param options A map of options.
   * @param args Command arguments.
   */
  private async execute(
    options: Record<string, unknown>,
    args: Array<unknown>,
  ): Promise<CommandResult> {
    if (this.defaultCommand) {
      const cmd = this.getCommand(this.defaultCommand, true);

      if (!cmd) {
        throw new DefaultCommandNotFoundError(
          this.defaultCommand,
          this.getCommands(),
        );
      }
      cmd._globalParent = this;

      return cmd.execute(options, args);
    }

    await this.executeGlobalAction(options, args);

    if (this.actionHandler) {
      await this.actionHandler(options, ...args);
    }

    return {
      options,
      args,
      cmd: this,
      literal: this.literalArgs,
    };
  }

  private async executeGlobalAction(
    options: Record<string, unknown>,
    args: Array<unknown>,
  ) {
    if (!this._noGlobals) {
      await this._parent?.executeGlobalAction(options, args);
    }
    await this.globalActionHandler?.(options, ...args);
  }

  /** Parse raw command line arguments. */
  protected parseOptions(
    ctx: ParseContext,
    options: Option[],
    {
      stopEarly = this._stopEarly,
      stopOnUnknown = false,
      dotted = true,
    }: ParseOptionsOptions = {},
  ): void {
    parseFlags(ctx, {
      stopEarly,
      stopOnUnknown,
      dotted,
      allowEmpty: this._allowEmpty,
      flags: options,
      ignoreDefaults: ctx.env,
      parse: (type: ArgumentValue) => this.parseType(type),
      option: (option: Option) => {
        if (option.action) {
          ctx.actions.push(option.action);
        }
      },
    });
  }

  /** Parse argument type. */
  protected parseType(type: ArgumentValue): unknown {
    const typeSettings: TypeDef | undefined = this.getType(type.type);

    if (!typeSettings) {
      throw new UnknownTypeError(
        type.type,
        this.getTypes().map((type) => type.name),
      );
    }

    return typeSettings.handler instanceof Type
      ? typeSettings.handler.parse(type)
      : typeSettings.handler(type);
  }

  /**
   * Read and validate environment variables.
   * @param ctx Parse context.
   * @param envVars env vars defined by the command.
   * @param validate when true, throws an error if a required env var is missing.
   */
  protected async parseEnvVars(
    ctx: ParseContext,
    envVars: Array<EnvVar>,
    validate = true,
  ): Promise<void> {
    for (const envVar of envVars) {
      const env = await this.findEnvVar(envVar.names);

      if (env) {
        const parseType = (value: string) => {
          return this.parseType({
            label: "Environment variable",
            type: envVar.type,
            name: env.name,
            value,
          });
        };

        const propertyName = underscoreToCamelCase(
          envVar.prefix
            ? envVar.names[0].replace(new RegExp(`^${envVar.prefix}`), "")
            : envVar.names[0],
        );

        if (envVar.details.list) {
          ctx.env[propertyName] = env.value
            .split(envVar.details.separator ?? ",")
            .map(parseType);
        } else {
          ctx.env[propertyName] = parseType(env.value);
        }

        if (envVar.value && typeof ctx.env[propertyName] !== "undefined") {
          ctx.env[propertyName] = envVar.value(ctx.env[propertyName]);
        }
      } else if (envVar.required && validate) {
        throw new MissingRequiredEnvVarError(envVar);
      }
    }
  }

  protected async findEnvVar(
    names: readonly string[],
  ): Promise<{ name: string; value: string } | undefined> {
    for (const name of names) {
      const status = await (dntShim.dntGlobalThis as any).Deno?.permissions.query({
        name: "env",
        variable: name,
      });

      if (!status || status.state === "granted") {
        const value = getEnv(name);

        if (value) {
          return { name, value };
        }
      }
    }

    return undefined;
  }

  /**
   * Parse command-line arguments.
   * @param ctx     Parse context.
   * @param options Parsed command line options.
   */
  protected parseArguments(
    ctx: ParseContext,
    options: Record<string, unknown>,
  ): TCommandArguments {
    const params: Array<unknown> = [];
    const args = ctx.unknown.slice();

    if (!this.hasArguments()) {
      if (args.length) {
        if (this.hasCommands(true)) {
          if (this.hasCommand(args[0], true)) {
            // e.g: command --global-foo --foo sub-command
            throw new TooManyArgumentsError(args);
          } else {
            throw new UnknownCommandError(args[0], this.getCommands());
          }
        } else {
          throw new NoArgumentsAllowedError(this.getPath());
        }
      }
    } else {
      if (!args.length) {
        const required = this.getArguments()
          .filter((expectedArg) => !expectedArg.optional)
          .map((expectedArg) => expectedArg.name);

        if (required.length) {
          const optionNames: string[] = Object.keys(options);
          const hasStandaloneOption = !!optionNames.find((name) =>
            this.getOption(name, true)?.standalone
          );

          if (!hasStandaloneOption) {
            throw new MissingArgumentsError(required);
          }
        }
      } else {
        for (const expectedArg of this.getArguments()) {
          if (!args.length) {
            if (expectedArg.optional) {
              break;
            }
            throw new MissingArgumentError(expectedArg.name);
          }

          let arg: unknown;

          const parseArgValue = (value: string) => {
            return expectedArg.list
              ? value.split(",").map((value) => parseArgType(value))
              : parseArgType(value);
          };

          const parseArgType = (value: string) => {
            return this.parseType({
              label: "Argument",
              type: expectedArg.type,
              name: expectedArg.name,
              value,
            });
          };

          if (expectedArg.variadic) {
            arg = args.splice(0, args.length).map((value) =>
              parseArgValue(value)
            );
          } else {
            arg = parseArgValue(args.shift() as string);
          }

          if (expectedArg.variadic && Array.isArray(arg)) {
            params.push(...arg);
          } else if (typeof arg !== "undefined") {
            params.push(arg);
          }
        }

        if (args.length) {
          throw new TooManyArgumentsError(args);
        }
      }
    }

    return params as TCommandArguments;
  }

  private handleError(error: unknown): never {
    this.throw(
      error instanceof FlagsValidationError
        ? new ValidationError(error.message)
        : error instanceof Error
        ? error
        : new Error(`[non-error-thrown] ${error}`),
    );
  }

  /**
   * Handle error. If `throwErrors` is enabled the error will be thrown,
   * otherwise a formatted error message will be printed and `exit(1)`
   * will be called. This will also trigger registered error handlers.
   *
   * @param error The error to handle.
   */
  public throw(error: Error): never {
    if (error instanceof ValidationError) {
      error.cmd = this as unknown as Command;
    }
    this.getErrorHandler()?.(error, this as unknown as Command);

    if (this.shouldThrowErrors() || !(error instanceof ValidationError)) {
      throw error;
    }
    this.showHelp();

    console.error(red(`  ${bold("error")}: ${error.message}\n`));

    exit(error instanceof ValidationError ? error.exitCode : 1);
  }

  /*****************************************************************************
   **** GETTER *****************************************************************
   *****************************************************************************/

  /** Get command name. */
  public getName(): string {
    return this._name;
  }

  /** Get parent command. */
  public getParent(): TParentCommand {
    return this._parent as TParentCommand;
  }

  /**
   * Get parent command from global executed command.
   * Be sure, to call this method only inside an action handler. Unless this or any child command was executed,
   * this method returns always undefined.
   */
  public getGlobalParent(): Command<any> | undefined {
    return this._globalParent;
  }

  /** Get main command. */
  public getMainCommand(): Command<any> {
    return this._parent?.getMainCommand() ?? this;
  }

  /** Get command name aliases. */
  public getAliases(): string[] {
    return this.aliases;
  }

  /**
   * Get full command path.
   *
   * @param name Override the main command name.
   */
  public getPath(name?: string): string {
    return this._parent
      ? this._parent.getPath(name) + " " + this._name
      : name || this._name;
  }

  /** Get arguments definition. E.g: <input-file:string> <output-file:string> */
  public getArgsDefinition(): string | undefined {
    return this.argsDefinition;
  }

  /**
   * Get argument by name.
   *
   * @param name Name of the argument.
   */
  public getArgument(name: string): Argument | undefined {
    return this.getArguments().find((arg) => arg.name === name);
  }

  /** Get arguments. */
  public getArguments(): Argument[] {
    if (!this.args.length && this.argsDefinition) {
      this.args = parseArgumentsDefinition(this.argsDefinition);
    }

    return this.args;
  }

  /** Check if command has arguments. */
  public hasArguments(): boolean {
    return !!this.argsDefinition;
  }

  /** Get command version. */
  public getVersion(): string | undefined {
    return this.getVersionHandler()?.call(this, this);
  }

  /** Get help handler method. */
  private getVersionHandler(): VersionHandler | undefined {
    return this.ver ?? this._parent?.getVersionHandler();
  }

  /** Get command description. */
  public getDescription(): string {
    // call description method only once
    return typeof this.desc === "function"
      ? this.desc = this.desc()
      : this.desc;
  }

  /** Get auto generated command usage. */
  public getUsage(): string {
    return this._usage ??
      [this.getArgsDefinition(), this.getRequiredOptionsDefinition()]
        .join(" ")
        .trim();
  }

  private getRequiredOptionsDefinition() {
    return this.getOptions()
      .filter((option) => option.required)
      .map((option) =>
        [findFlag(option.flags), option.typeDefinition].filter((v) => v)
          .join(" ")
          .trim()
      )
      .join(" ");
  }

  /** Get short command description. This is the first line of the description. */
  public getShortDescription(): string {
    return getDescription(this.getDescription(), true);
  }

  /** Get original command-line arguments. */
  public getRawArgs(): string[] {
    return this.rawArgs;
  }

  /** Get all arguments defined after the double dash. */
  public getLiteralArgs(): string[] {
    return this.literalArgs;
  }

  /** Output generated help without exiting. */
  public showVersion(): void {
    console.log(this.getVersion());
  }

  /** Returns command name, version and meta data. */
  public getLongVersion(): string {
    return `${bold(this.getMainCommand().getName())} ${
      brightBlue(this.getVersion() ?? "")
    }` +
      Object.entries(this.getMeta()).map(
        ([k, v]) => `\n${bold(k)} ${brightBlue(v)}`,
      ).join("");
  }

  /** Outputs command name, version and meta data. */
  public showLongVersion(): void {
    console.log(this.getLongVersion());
  }

  /** Output generated help without exiting. */
  public showHelp(options?: HelpOptions): void {
    console.log(this.getHelp(options));
  }

  /** Get generated help. */
  public getHelp(options?: HelpOptions): string {
    this.registerDefaults();
    return this.getHelpHandler().call(this, this, options ?? {});
  }

  /** Get help handler method. */
  private getHelpHandler(): HelpHandler {
    return this._help ?? this._parent?.getHelpHandler() as HelpHandler;
  }

  private exit(code = 0) {
    if (this.shouldExit()) {
      exit(code);
    }
  }

  /*****************************************************************************
   **** Options GETTER *********************************************************
   *****************************************************************************/

  /**
   * Checks whether the command has options or not.
   *
   * @param hidden Include hidden options.
   */
  public hasOptions(hidden?: boolean): boolean {
    return this.getOptions(hidden).length > 0;
  }

  /**
   * Get options.
   *
   * @param hidden Include hidden options.
   */
  public getOptions(hidden?: boolean): Option[] {
    return this.getGlobalOptions(hidden).concat(this.getBaseOptions(hidden));
  }

  /**
   * Get base options.
   *
   * @param hidden Include hidden options.
   */
  public getBaseOptions(hidden?: boolean): Option[] {
    if (!this.options.length) {
      return [];
    }

    return hidden
      ? this.options.slice(0)
      : this.options.filter((opt) => !opt.hidden);
  }

  /**
   * Get global options.
   *
   * @param hidden Include hidden options.
   */
  public getGlobalOptions(hidden?: boolean): Option[] {
    const helpOption = this.getHelpOption();
    const getGlobals = (
      cmd: Command<any>,
      noGlobals: boolean,
      options: Option[] = [],
      names: string[] = [],
    ): Option[] => {
      if (cmd.options.length) {
        for (const option of cmd.options) {
          if (
            option.global &&
            !this.options.find((opt) => opt.name === option.name) &&
            names.indexOf(option.name) === -1 &&
            (hidden || !option.hidden)
          ) {
            if (noGlobals && option !== helpOption) {
              continue;
            }

            names.push(option.name);
            options.push(option);
          }
        }
      }

      return cmd._parent
        ? getGlobals(
          cmd._parent,
          noGlobals || cmd._noGlobals,
          options,
          names,
        )
        : options;
    };

    return this._parent ? getGlobals(this._parent, this._noGlobals) : [];
  }

  /**
   * Checks whether the command has an option with given name or not.
   *
   * @param name Name of the option. Must be in param-case.
   * @param hidden Include hidden options.
   */
  public hasOption(name: string, hidden?: boolean): boolean {
    return !!this.getOption(name, hidden);
  }

  /**
   * Get option by name.
   *
   * @param name Name of the option. Must be in param-case.
   * @param hidden Include hidden options.
   */
  public getOption(name: string, hidden?: boolean): Option | undefined {
    return this.getBaseOption(name, hidden) ??
      this.getGlobalOption(name, hidden);
  }

  /**
   * Get base option by name.
   *
   * @param name Name of the option. Must be in param-case.
   * @param hidden Include hidden options.
   */
  public getBaseOption(name: string, hidden?: boolean): Option | undefined {
    const option = this.options.find((option) =>
      option.name === name || option.aliases?.includes(name)
    );

    return option && (hidden || !option.hidden) ? option : undefined;
  }

  /**
   * Get global option from parent commands by name.
   *
   * @param name Name of the option. Must be in param-case.
   * @param hidden Include hidden options.
   */
  public getGlobalOption(name: string, hidden?: boolean): Option | undefined {
    const helpOption = this.getHelpOption();
    const getGlobalOption = (
      parent: Command,
      noGlobals: boolean,
    ): Option | undefined => {
      const option: Option | undefined = parent.getBaseOption(
        name,
        hidden,
      );

      if (!option?.global) {
        return parent._parent && getGlobalOption(
          parent._parent,
          noGlobals || parent._noGlobals,
        );
      }
      if (noGlobals && option !== helpOption) {
        return;
      }

      return option;
    };

    return this._parent && getGlobalOption(
      this._parent,
      this._noGlobals,
    );
  }

  /**
   * Remove option by name.
   *
   * @param name Name of the option. Must be in param-case.
   */
  public removeOption(name: string): Option | undefined {
    const index = this.options.findIndex((option) => option.name === name);

    if (index === -1) {
      return;
    }

    return this.options.splice(index, 1)[0];
  }

  /**
   * Checks whether the command has sub-commands or not.
   *
   * @param hidden Include hidden commands.
   */
  public hasCommands(hidden?: boolean): boolean {
    return this.getCommands(hidden).length > 0;
  }

  /**
   * Get commands.
   *
   * @param hidden Include hidden commands.
   */
  public getCommands(hidden?: boolean): Array<Command<any>> {
    return this.getGlobalCommands(hidden).concat(this.getBaseCommands(hidden));
  }

  /**
   * Get base commands.
   *
   * @param hidden Include hidden commands.
   */
  public getBaseCommands(hidden?: boolean): Array<Command<any>> {
    const commands = Array.from(this.commands.values());
    return hidden ? commands : commands.filter((cmd) => !cmd.isHidden);
  }

  /**
   * Get global commands.
   *
   * @param hidden Include hidden commands.
   */
  public getGlobalCommands(hidden?: boolean): Array<Command<any>> {
    const getCommands = (
      command: Command<any>,
      noGlobals: boolean,
      commands: Array<Command<any>> = [],
      names: string[] = [],
    ): Array<Command<any>> => {
      if (command.commands.size) {
        for (const [_, cmd] of command.commands) {
          if (
            cmd.isGlobal &&
            this !== cmd &&
            !this.commands.has(cmd._name) &&
            names.indexOf(cmd._name) === -1 &&
            (hidden || !cmd.isHidden)
          ) {
            if (noGlobals && cmd?.getName() !== "help") {
              continue;
            }

            names.push(cmd._name);
            commands.push(cmd);
          }
        }
      }

      return command._parent
        ? getCommands(
          command._parent,
          noGlobals || command._noGlobals,
          commands,
          names,
        )
        : commands;
    };

    return this._parent ? getCommands(this._parent, this._noGlobals) : [];
  }

  /**
   * Checks whether a child command exists by given name or alias.
   *
   * @param name Name or alias of the command.
   * @param hidden Include hidden commands.
   */
  public hasCommand(name: string, hidden?: boolean): boolean {
    return !!this.getCommand(name, hidden);
  }

  /**
   * Get command by name or alias.
   *
   * @param name Name or alias of the command.
   * @param hidden Include hidden commands.
   */
  public getCommand<TCommand extends Command<any>>(
    name: string,
    hidden?: boolean,
  ): TCommand | undefined {
    return this.getBaseCommand(name, hidden) ??
      this.getGlobalCommand(name, hidden);
  }

  /**
   * Get base command by name or alias.
   *
   * @param name Name or alias of the command.
   * @param hidden Include hidden commands.
   */
  public getBaseCommand<TCommand extends Command<any>>(
    name: string,
    hidden?: boolean,
  ): TCommand | undefined {
    for (const cmd of this.commands.values()) {
      if (cmd._name === name || cmd.aliases.includes(name)) {
        return (cmd && (hidden || !cmd.isHidden) ? cmd : undefined) as
          | TCommand
          | undefined;
      }
    }
  }

  /**
   * Get global command by name or alias.
   *
   * @param name Name or alias of the command.
   * @param hidden Include hidden commands.
   */
  public getGlobalCommand<TCommand extends Command<any>>(
    name: string,
    hidden?: boolean,
  ): TCommand | undefined {
    const getGlobalCommand = (
      parent: Command,
      noGlobals: boolean,
    ): Command | undefined => {
      const cmd: Command | undefined = parent.getBaseCommand(name, hidden);

      if (!cmd?.isGlobal) {
        return parent._parent &&
          getGlobalCommand(parent._parent, noGlobals || parent._noGlobals);
      }
      if (noGlobals && cmd.getName() !== "help") {
        return;
      }

      return cmd;
    };

    return this._parent &&
      getGlobalCommand(this._parent, this._noGlobals) as TCommand;
  }

  /**
   * Remove sub-command by name or alias.
   *
   * @param name Name or alias of the command.
   */
  public removeCommand(name: string): Command<any> | undefined {
    const command = this.getBaseCommand(name, true);

    if (command) {
      this.commands.delete(command._name);
    }

    return command;
  }

  /** Get types. */
  public getTypes(): Array<TypeDef> {
    return this.getGlobalTypes().concat(this.getBaseTypes());
  }

  /** Get base types. */
  public getBaseTypes(): Array<TypeDef> {
    return Array.from(this.types.values());
  }

  /** Get global types. */
  public getGlobalTypes(): Array<TypeDef> {
    const getTypes = (
      cmd: Command<any> | undefined,
      types: Array<TypeDef> = [],
      names: Array<string> = [],
    ): Array<TypeDef> => {
      if (cmd) {
        if (cmd.types.size) {
          cmd.types.forEach((type: TypeDef) => {
            if (
              type.global &&
              !this.types.has(type.name) &&
              names.indexOf(type.name) === -1
            ) {
              names.push(type.name);
              types.push(type);
            }
          });
        }

        return getTypes(cmd._parent, types, names);
      }

      return types;
    };

    return getTypes(this._parent);
  }

  /**
   * Get type by name.
   *
   * @param name Name of the type.
   */
  public getType(name: string): TypeDef | undefined {
    return this.getBaseType(name) ?? this.getGlobalType(name);
  }

  /**
   * Get base type by name.
   *
   * @param name Name of the type.
   */
  public getBaseType(name: string): TypeDef | undefined {
    return this.types.get(name);
  }

  /**
   * Get global type by name.
   *
   * @param name Name of the type.
   */
  public getGlobalType(name: string): TypeDef | undefined {
    if (!this._parent) {
      return;
    }

    const cmd: TypeDef | undefined = this._parent.getBaseType(name);

    if (!cmd?.global) {
      return this._parent.getGlobalType(name);
    }

    return cmd;
  }

  /** Get completions. */
  public getCompletions(): Completion<
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any
  >[] {
    return this.getGlobalCompletions().concat(this.getBaseCompletions());
  }

  /** Get base completions. */
  public getBaseCompletions(): Completion[] {
    return Array.from(this.completions.values());
  }

  /** Get global completions. */
  public getGlobalCompletions(): Completion[] {
    const getCompletions = (
      cmd: Command<any> | undefined,
      completions: Completion[] = [],
      names: string[] = [],
    ): Completion[] => {
      if (cmd) {
        if (cmd.completions.size) {
          cmd.completions.forEach((completion: Completion) => {
            if (
              completion.global &&
              !this.completions.has(completion.name) &&
              names.indexOf(completion.name) === -1
            ) {
              names.push(completion.name);
              completions.push(completion);
            }
          });
        }

        return getCompletions(cmd._parent, completions, names);
      }

      return completions;
    };

    return getCompletions(this._parent);
  }

  /**
   * Get completion by name.
   *
   * @param name Name of the completion.
   */
  public getCompletion(name: string): Completion | undefined {
    return this.getBaseCompletion(name) ?? this.getGlobalCompletion(name);
  }

  /**
   * Get base completion by name.
   *
   * @param name Name of the completion.
   */
  public getBaseCompletion(name: string): Completion | undefined {
    return this.completions.get(name);
  }

  /**
   * Get global completions by name.
   *
   * @param name Name of the completion.
   */
  public getGlobalCompletion(name: string): Completion | undefined {
    if (!this._parent) {
      return;
    }

    const completion: Completion | undefined = this._parent.getBaseCompletion(
      name,
    );

    if (!completion?.global) {
      return this._parent.getGlobalCompletion(name);
    }

    return completion;
  }

  /**
   * Checks whether the command has environment variables or not.
   *
   * @param hidden Include hidden environment variable.
   */
  public hasEnvVars(hidden?: boolean): boolean {
    return this.getEnvVars(hidden).length > 0;
  }

  /**
   * Get environment variables.
   *
   * @param hidden Include hidden environment variable.
   */
  public getEnvVars(hidden?: boolean): EnvVar[] {
    return this.getGlobalEnvVars(hidden).concat(this.getBaseEnvVars(hidden));
  }

  /**
   * Get base environment variables.
   *
   * @param hidden Include hidden environment variable.
   */
  public getBaseEnvVars(hidden?: boolean): EnvVar[] {
    if (!this.envVars.length) {
      return [];
    }

    return hidden
      ? this.envVars.slice(0)
      : this.envVars.filter((env) => !env.hidden);
  }

  /**
   * Get global environment variables.
   *
   * @param hidden Include hidden environment variable.
   */
  public getGlobalEnvVars(hidden?: boolean): EnvVar[] {
    if (this._noGlobals) {
      return [];
    }

    const getEnvVars = (
      cmd: Command<any> | undefined,
      envVars: EnvVar[] = [],
      names: string[] = [],
    ): EnvVar[] => {
      if (cmd) {
        if (cmd.envVars.length) {
          cmd.envVars.forEach((envVar: EnvVar) => {
            if (
              envVar.global &&
              !this.envVars.find((env) => env.names[0] === envVar.names[0]) &&
              names.indexOf(envVar.names[0]) === -1 &&
              (hidden || !envVar.hidden)
            ) {
              names.push(envVar.names[0]);
              envVars.push(envVar);
            }
          });
        }

        return getEnvVars(cmd._parent, envVars, names);
      }

      return envVars;
    };

    return getEnvVars(this._parent);
  }

  /**
   * Checks whether the command has an environment variable with given name or not.
   *
   * @param name Name of the environment variable.
   * @param hidden Include hidden environment variable.
   */
  public hasEnvVar(name: string, hidden?: boolean): boolean {
    return !!this.getEnvVar(name, hidden);
  }

  /**
   * Get environment variable by name.
   *
   * @param name Name of the environment variable.
   * @param hidden Include hidden environment variable.
   */
  public getEnvVar(name: string, hidden?: boolean): EnvVar | undefined {
    return this.getBaseEnvVar(name, hidden) ??
      this.getGlobalEnvVar(name, hidden);
  }

  /**
   * Get base environment variable by name.
   *
   * @param name Name of the environment variable.
   * @param hidden Include hidden environment variable.
   */
  public getBaseEnvVar(name: string, hidden?: boolean): EnvVar | undefined {
    const envVar: EnvVar | undefined = this.envVars.find((env) =>
      env.names.indexOf(name) !== -1
    );

    return envVar && (hidden || !envVar.hidden) ? envVar : undefined;
  }

  /**
   * Get global environment variable by name.
   *
   * @param name Name of the environment variable.
   * @param hidden Include hidden environment variable.
   */
  public getGlobalEnvVar(name: string, hidden?: boolean): EnvVar | undefined {
    if (!this._parent || this._noGlobals) {
      return;
    }

    const envVar: EnvVar | undefined = this._parent.getBaseEnvVar(
      name,
      hidden,
    );

    if (!envVar?.global) {
      return this._parent.getGlobalEnvVar(name, hidden);
    }

    return envVar;
  }

  /** Checks whether the command has examples or not. */
  public hasExamples(): boolean {
    return this.examples.length > 0;
  }

  /** Get all examples. */
  public getExamples(): Example[] {
    return this.examples;
  }

  /** Checks whether the command has an example with given name or not. */
  public hasExample(name: string): boolean {
    return !!this.getExample(name);
  }

  /** Get example with given name. */
  public getExample(name: string): Example | undefined {
    return this.examples.find((example) => example.name === name);
  }

  private getHelpOption(): Option | undefined {
    return this._helpOption ?? this._parent?.getHelpOption();
  }
}

function findFlag(flags: Array<string>): string {
  for (const flag of flags) {
    if (flag.startsWith("--")) {
      return flag;
    }
  }
  return flags[0];
}

interface DefaultOption {
  flags: string;
  desc?: string;
  opts?: OptionOptions;
}

interface ParseContext extends ParseFlagsContext<Record<string, unknown>> {
  actions: Array<ActionHandler>;
  env: Record<string, unknown>;
}

interface ParseOptionsOptions {
  stopEarly?: boolean;
  stopOnUnknown?: boolean;
  dotted?: boolean;
}
