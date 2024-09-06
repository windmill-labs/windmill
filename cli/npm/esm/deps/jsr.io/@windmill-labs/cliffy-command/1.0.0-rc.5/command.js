// deno-lint-ignore-file no-explicit-any
import * as dntShim from "../../../../../_dnt.shims.js";
import { parseFlags, UnknownTypeError, ValidationError as FlagsValidationError, } from "../../cliffy-flags/1.0.0-rc.5/mod.js";
import { bold, brightBlue, red } from "../../../@std/fmt/0.225.6/colors.js";
import { CommandNotFoundError, DefaultCommandNotFoundError, DuplicateCommandAliasError, DuplicateCommandNameError, DuplicateCompletionError, DuplicateEnvVarError, DuplicateExampleError, DuplicateOptionNameError, DuplicateTypeError, MissingArgumentError, MissingArgumentsError, MissingCommandNameError, MissingRequiredEnvVarError, NoArgumentsAllowedError, TooManyArgumentsError, TooManyEnvVarValuesError, UnexpectedOptionalEnvVarValueError, UnexpectedVariadicEnvVarValueError, UnknownCommandError, ValidationError, } from "./_errors.js";
import { exit } from "../../cliffy-internal/1.0.0-rc.5/runtime/exit.js";
import { getArgs } from "../../cliffy-internal/1.0.0-rc.5/runtime/get_args.js";
import { getEnv } from "../../cliffy-internal/1.0.0-rc.5/runtime/get_env.js";
import { getDescription, parseArgumentsDefinition, splitArguments, underscoreToCamelCase, } from "./_utils.js";
import { HelpGenerator } from "./help/_help_generator.js";
import { Type } from "./type.js";
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
export class Command {
    constructor() {
        Object.defineProperty(this, "types", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: new Map()
        });
        Object.defineProperty(this, "rawArgs", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
        Object.defineProperty(this, "literalArgs", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
        Object.defineProperty(this, "_name", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: "COMMAND"
        });
        Object.defineProperty(this, "_parent", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "_globalParent", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "ver", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "desc", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: ""
        });
        Object.defineProperty(this, "_usage", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "actionHandler", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "globalActionHandler", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "options", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
        Object.defineProperty(this, "commands", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: new Map()
        });
        Object.defineProperty(this, "examples", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
        Object.defineProperty(this, "envVars", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
        Object.defineProperty(this, "aliases", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
        Object.defineProperty(this, "completions", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: new Map()
        });
        Object.defineProperty(this, "cmd", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: this
        });
        Object.defineProperty(this, "argsDefinition", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "throwOnError", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: false
        });
        Object.defineProperty(this, "_allowEmpty", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: false
        });
        Object.defineProperty(this, "_stopEarly", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: false
        });
        Object.defineProperty(this, "defaultCommand", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "_useRawArgs", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: false
        });
        Object.defineProperty(this, "args", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
        Object.defineProperty(this, "isHidden", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: false
        });
        Object.defineProperty(this, "isGlobal", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: false
        });
        Object.defineProperty(this, "hasDefaults", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: false
        });
        Object.defineProperty(this, "_versionOptions", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "_helpOptions", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "_versionOption", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "_helpOption", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "_help", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "_shouldExit", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "_meta", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: {}
        });
        Object.defineProperty(this, "_groupName", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: null
        });
        Object.defineProperty(this, "_noGlobals", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: false
        });
        Object.defineProperty(this, "errorHandler", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
    }
    versionOption(flags, desc, opts) {
        this._versionOptions = flags === false ? flags : {
            flags,
            desc,
            opts: typeof opts === "function" ? { action: opts } : opts,
        };
        return this;
    }
    helpOption(flags, desc, opts) {
        this._helpOptions = flags === false ? flags : {
            flags,
            desc,
            opts: typeof opts === "function" ? { action: opts } : opts,
        };
        return this;
    }
    /**
     * Add new sub-command.
     * @param nameAndArguments  Command definition. E.g: `my-command <input-file:string> <output-file:string>`
     * @param cmdOrDescription  The description of the new child command.
     * @param override          Override existing child command.
     */
    command(nameAndArguments, cmdOrDescription, override) {
        this.reset();
        const result = splitArguments(nameAndArguments);
        const name = result.flags.shift();
        const aliases = result.flags;
        if (!name) {
            throw new MissingCommandNameError();
        }
        if (this.getBaseCommand(name, true)) {
            if (!override) {
                throw new DuplicateCommandNameError(name);
            }
            this.removeCommand(name);
        }
        let description;
        let cmd;
        if (typeof cmdOrDescription === "string") {
            description = cmdOrDescription;
        }
        if (cmdOrDescription instanceof Command) {
            cmd = cmdOrDescription.reset();
        }
        else {
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
        aliases.forEach((alias) => cmd.alias(alias));
        this.commands.set(name, cmd);
        this.select(name);
        return this;
    }
    /**
     * Add new command alias.
     *
     * @param alias Tha name of the alias.
     */
    alias(alias) {
        if (this.cmd._name === alias || this.cmd.aliases.includes(alias)) {
            throw new DuplicateCommandAliasError(alias);
        }
        this.cmd.aliases.push(alias);
        return this;
    }
    /** Reset internal command reference to main command. */
    reset() {
        this._groupName = null;
        this.cmd = this;
        return this;
    }
    /**
     * Set internal command pointer to child command with given name.
     * @param name The name of the command to select.
     */
    select(name) {
        const cmd = this.getBaseCommand(name, true);
        if (!cmd) {
            throw new CommandNotFoundError(name, this.getBaseCommands(true));
        }
        this.cmd = cmd;
        return this;
    }
    /*****************************************************************************
     **** SUB HANDLER ************************************************************
     *****************************************************************************/
    /** Set command name. Used in auto generated help and shell completions */
    name(name) {
        this.cmd._name = name;
        return this;
    }
    /**
     * Set command version.
     *
     * @param version Semantic version string string or method that returns the version string.
     */
    version(version) {
        if (typeof version === "string") {
            this.cmd.ver = () => version;
        }
        else if (typeof version === "function") {
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
    meta(name, value) {
        this.cmd._meta[name] = value;
        return this;
    }
    getMeta(name) {
        return typeof name === "undefined" ? this._meta : this._meta[name];
    }
    /**
     * Set command help.
     *
     * @param help Help string, method, or config for generator that returns the help string.
     */
    help(help) {
        if (typeof help === "string") {
            this.cmd._help = () => help;
        }
        else if (typeof help === "function") {
            this.cmd._help = help;
        }
        else {
            this.cmd._help = (cmd, options) => HelpGenerator.generate(cmd, { ...help, ...options });
        }
        return this;
    }
    /**
     * Set the long command description.
     *
     * @param description The command description.
     */
    description(description) {
        this.cmd.desc = description;
        return this;
    }
    /**
     * Set the command usage. Defaults to arguments.
     *
     * @param usage The command usage.
     */
    usage(usage) {
        this.cmd._usage = usage;
        return this;
    }
    /** Hide command from help, completions, etc. */
    hidden() {
        this.cmd.isHidden = true;
        return this;
    }
    /** Make command globally available. */
    global() {
        this.cmd.isGlobal = true;
        return this;
    }
    /**
     * Set command arguments.
     *
     * Syntax: `<requiredArg:string> [optionalArg: number] [...restArgs:string]`
     */
    arguments(args) {
        this.cmd.argsDefinition = args;
        return this;
    }
    /**
     * Set command callback method.
     *
     * @param fn Command action handler.
     */
    action(fn) {
        this.cmd.actionHandler = fn;
        return this;
    }
    /**
     * Set command callback method.
     *
     * @param fn Command action handler.
     */
    globalAction(fn) {
        this.cmd.globalActionHandler = fn;
        return this;
    }
    /**
     * Don't throw an error if the command was called without arguments.
     *
     * @param allowEmpty Enable/disable allow empty.
     */
    allowEmpty(allowEmpty) {
        this.cmd._allowEmpty = allowEmpty !== false;
        return this;
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
    stopEarly(stopEarly = true) {
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
    useRawArgs(useRawArgs = true) {
        this.cmd._useRawArgs = useRawArgs;
        return this;
    }
    /**
     * Set default command. The default command is executed when the program
     * was called without any argument and if no action handler is registered.
     *
     * @param name Name of the default command.
     */
    default(name) {
        this.cmd.defaultCommand = name;
        return this;
    }
    globalType(name, handler, options) {
        return this.type(name, handler, { ...options, global: true });
    }
    /**
     * Register custom type.
     *
     * @param name    The name of the type.
     * @param handler The callback method to parse the type.
     * @param options Type options.
     */
    type(name, handler, options) {
        if (this.cmd.types.get(name) && !options?.override) {
            throw new DuplicateTypeError(name);
        }
        this.cmd.types.set(name, {
            ...options,
            name,
            handler: handler,
        });
        if (handler instanceof Type &&
            (typeof handler.complete !== "undefined" ||
                typeof handler.values !== "undefined")) {
            const completeHandler = (cmd, parent) => handler.complete?.(cmd, parent) || [];
            this.complete(name, completeHandler, options);
        }
        return this;
    }
    /**
     * Register global complete handler.
     *
     * @param name      The name of the completion.
     * @param complete  The callback method to complete the type.
     * @param options   Complete options.
     */
    globalComplete(name, complete, options) {
        return this.complete(name, complete, { ...options, global: true });
    }
    complete(name, complete, options) {
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
    throwErrors() {
        this.cmd.throwOnError = true;
        return this;
    }
    /**
     * Set custom error handler.
     *
     * @param handler Error handler callback function.
     */
    error(handler) {
        this.cmd.errorHandler = handler;
        return this;
    }
    /** Get error handler callback function. */
    getErrorHandler() {
        return this.errorHandler ?? this._parent?.errorHandler;
    }
    /**
     * Same as `.throwErrors()` but also prevents calling `exit()` after
     * printing help or version with the --help and --version option.
     */
    noExit() {
        this.cmd._shouldExit = false;
        this.throwErrors();
        return this;
    }
    /**
     * Disable inheriting global commands, options and environment variables from
     * parent commands.
     */
    noGlobals() {
        this.cmd._noGlobals = true;
        return this;
    }
    /** Check whether the command should throw errors or exit. */
    shouldThrowErrors() {
        return this.throwOnError || !!this._parent?.shouldThrowErrors();
    }
    /** Check whether the command should exit after printing help or version. */
    shouldExit() {
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
    group(name) {
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
    globalOption(flags, desc, opts) {
        if (typeof opts === "function") {
            return this.option(flags, desc, { value: opts, global: true });
        }
        return this.option(flags, desc, { ...opts, global: true });
    }
    option(flags, desc, opts) {
        if (typeof opts === "function") {
            opts = { value: opts };
        }
        const result = splitArguments(flags);
        const args = result.typeDefinition
            ? parseArgumentsDefinition(result.typeDefinition)
            : [];
        const option = {
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
                }
                else {
                    throw new DuplicateOptionNameError(name, this.getPath());
                }
            }
            if (!option.name && isLong) {
                option.name = name;
            }
            else if (!option.aliases) {
                option.aliases = [name];
            }
            else {
                option.aliases.push(name);
            }
        }
        if (option.prepend) {
            this.cmd.options.unshift(option);
        }
        else {
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
    example(name, description) {
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
    globalEnv(name, description, options) {
        return this.env(name, description, { ...options, global: true });
    }
    env(name, description, options) {
        const result = splitArguments(name);
        if (!result.typeDefinition) {
            result.typeDefinition = "<value:boolean>";
        }
        if (result.flags.some((envName) => this.cmd.getBaseEnvVar(envName, true))) {
            throw new DuplicateEnvVarError(name);
        }
        const details = parseArgumentsDefinition(result.typeDefinition);
        if (details.length > 1) {
            throw new TooManyEnvVarValuesError(name);
        }
        else if (details.length && details[0].optional) {
            throw new UnexpectedOptionalEnvVarValueError(name);
        }
        else if (details.length && details[0].variadic) {
            throw new UnexpectedVariadicEnvVarValueError(name);
        }
        this.cmd.envVars.push({
            name: result.flags[0],
            names: result.flags,
            description,
            type: details[0].type,
            details: details.shift(),
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
    parse(args = getArgs()) {
        const ctx = {
            unknown: args.slice(),
            flags: {},
            env: {},
            literal: [],
            stopEarly: false,
            stopOnUnknown: false,
            defaults: {},
            actions: [],
        };
        return this.parseCommand(ctx);
    }
    async parseCommand(ctx) {
        try {
            this.reset();
            this.registerDefaults();
            this.rawArgs = ctx.unknown.slice();
            if (this._useRawArgs) {
                await this.parseEnvVars(ctx, this.envVars);
                return await this.execute(ctx.env, ctx.unknown);
            }
            let preParseGlobals = false;
            let subCommand;
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
                await Promise.all(ctx.actions.map((action) => action.call(this, options, ...args)));
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
        }
        catch (error) {
            this.handleError(error);
        }
    }
    getSubCommand(ctx) {
        const subCommand = this.getCommand(ctx.unknown[0], true);
        if (subCommand) {
            ctx.unknown.shift();
        }
        return subCommand;
    }
    async parseGlobalOptionsAndEnvVars(ctx) {
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
    async parseOptionsAndEnvVars(ctx, preParseGlobals) {
        const helpOption = this.getHelpOption();
        const isVersionOption = this._versionOption?.flags.includes(ctx.unknown[0]);
        const isHelpOption = helpOption && ctx.flags?.[helpOption.name] === true;
        // Parse env vars.
        const envVars = preParseGlobals
            ? this.envVars.filter((envVar) => !envVar.global)
            : this.getEnvVars(true);
        await this.parseEnvVars(ctx, envVars, !isHelpOption && !isVersionOption);
        // Parse options.
        const options = this.getOptions(true);
        this.parseOptions(ctx, options);
    }
    /** Register default options like `--version` and `--help`. */
    registerDefaults() {
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
            this.option(this._versionOptions?.flags || "-V, --version", this._versionOptions?.desc ||
                "Show the version number for this program.", {
                standalone: true,
                prepend: true,
                action: async function () {
                    const long = this.getRawArgs().includes(`--${this._versionOption?.name}`);
                    if (long) {
                        await checkVersion(this);
                        this.showLongVersion();
                    }
                    else {
                        this.showVersion();
                    }
                    this.exit();
                },
                ...(this._versionOptions?.opts ?? {}),
            });
            this._versionOption = this.options[0];
        }
        if (this._helpOptions !== false) {
            this.option(this._helpOptions?.flags || "-h, --help", this._helpOptions?.desc || "Show this help.", {
                standalone: true,
                global: true,
                prepend: true,
                action: async function () {
                    const long = this.getRawArgs().includes(`--${this.getHelpOption()?.name}`);
                    await checkVersion(this);
                    this.showHelp({ long });
                    this.exit();
                },
                ...(this._helpOptions?.opts ?? {}),
            });
            this._helpOption = this.options[0];
        }
        return this;
    }
    /**
     * Execute command.
     * @param options A map of options.
     * @param args Command arguments.
     */
    async execute(options, args) {
        if (this.defaultCommand) {
            const cmd = this.getCommand(this.defaultCommand, true);
            if (!cmd) {
                throw new DefaultCommandNotFoundError(this.defaultCommand, this.getCommands());
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
    async executeGlobalAction(options, args) {
        if (!this._noGlobals) {
            await this._parent?.executeGlobalAction(options, args);
        }
        await this.globalActionHandler?.(options, ...args);
    }
    /** Parse raw command line arguments. */
    parseOptions(ctx, options, { stopEarly = this._stopEarly, stopOnUnknown = false, dotted = true, } = {}) {
        parseFlags(ctx, {
            stopEarly,
            stopOnUnknown,
            dotted,
            allowEmpty: this._allowEmpty,
            flags: options,
            ignoreDefaults: ctx.env,
            parse: (type) => this.parseType(type),
            option: (option) => {
                if (option.action) {
                    ctx.actions.push(option.action);
                }
            },
        });
    }
    /** Parse argument type. */
    parseType(type) {
        const typeSettings = this.getType(type.type);
        if (!typeSettings) {
            throw new UnknownTypeError(type.type, this.getTypes().map((type) => type.name));
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
    async parseEnvVars(ctx, envVars, validate = true) {
        for (const envVar of envVars) {
            const env = await this.findEnvVar(envVar.names);
            if (env) {
                const parseType = (value) => {
                    return this.parseType({
                        label: "Environment variable",
                        type: envVar.type,
                        name: env.name,
                        value,
                    });
                };
                const propertyName = underscoreToCamelCase(envVar.prefix
                    ? envVar.names[0].replace(new RegExp(`^${envVar.prefix}`), "")
                    : envVar.names[0]);
                if (envVar.details.list) {
                    ctx.env[propertyName] = env.value
                        .split(envVar.details.separator ?? ",")
                        .map(parseType);
                }
                else {
                    ctx.env[propertyName] = parseType(env.value);
                }
                if (envVar.value && typeof ctx.env[propertyName] !== "undefined") {
                    ctx.env[propertyName] = envVar.value(ctx.env[propertyName]);
                }
            }
            else if (envVar.required && validate) {
                throw new MissingRequiredEnvVarError(envVar);
            }
        }
    }
    async findEnvVar(names) {
        for (const name of names) {
            const status = await dntShim.dntGlobalThis.Deno?.permissions.query({
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
    parseArguments(ctx, options) {
        const params = [];
        const args = ctx.unknown.slice();
        if (!this.hasArguments()) {
            if (args.length) {
                if (this.hasCommands(true)) {
                    if (this.hasCommand(args[0], true)) {
                        // e.g: command --global-foo --foo sub-command
                        throw new TooManyArgumentsError(args);
                    }
                    else {
                        throw new UnknownCommandError(args[0], this.getCommands());
                    }
                }
                else {
                    throw new NoArgumentsAllowedError(this.getPath());
                }
            }
        }
        else {
            if (!args.length) {
                const required = this.getArguments()
                    .filter((expectedArg) => !expectedArg.optional)
                    .map((expectedArg) => expectedArg.name);
                if (required.length) {
                    const optionNames = Object.keys(options);
                    const hasStandaloneOption = !!optionNames.find((name) => this.getOption(name, true)?.standalone);
                    if (!hasStandaloneOption) {
                        throw new MissingArgumentsError(required);
                    }
                }
            }
            else {
                for (const expectedArg of this.getArguments()) {
                    if (!args.length) {
                        if (expectedArg.optional) {
                            break;
                        }
                        throw new MissingArgumentError(expectedArg.name);
                    }
                    let arg;
                    const parseArgValue = (value) => {
                        return expectedArg.list
                            ? value.split(",").map((value) => parseArgType(value))
                            : parseArgType(value);
                    };
                    const parseArgType = (value) => {
                        return this.parseType({
                            label: "Argument",
                            type: expectedArg.type,
                            name: expectedArg.name,
                            value,
                        });
                    };
                    if (expectedArg.variadic) {
                        arg = args.splice(0, args.length).map((value) => parseArgValue(value));
                    }
                    else {
                        arg = parseArgValue(args.shift());
                    }
                    if (expectedArg.variadic && Array.isArray(arg)) {
                        params.push(...arg);
                    }
                    else if (typeof arg !== "undefined") {
                        params.push(arg);
                    }
                }
                if (args.length) {
                    throw new TooManyArgumentsError(args);
                }
            }
        }
        return params;
    }
    handleError(error) {
        this.throw(error instanceof FlagsValidationError
            ? new ValidationError(error.message)
            : error instanceof Error
                ? error
                : new Error(`[non-error-thrown] ${error}`));
    }
    /**
     * Handle error. If `throwErrors` is enabled the error will be thrown,
     * otherwise a formatted error message will be printed and `exit(1)`
     * will be called. This will also trigger registered error handlers.
     *
     * @param error The error to handle.
     */
    throw(error) {
        if (error instanceof ValidationError) {
            error.cmd = this;
        }
        this.getErrorHandler()?.(error, this);
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
    getName() {
        return this._name;
    }
    /** Get parent command. */
    getParent() {
        return this._parent;
    }
    /**
     * Get parent command from global executed command.
     * Be sure, to call this method only inside an action handler. Unless this or any child command was executed,
     * this method returns always undefined.
     */
    getGlobalParent() {
        return this._globalParent;
    }
    /** Get main command. */
    getMainCommand() {
        return this._parent?.getMainCommand() ?? this;
    }
    /** Get command name aliases. */
    getAliases() {
        return this.aliases;
    }
    /**
     * Get full command path.
     *
     * @param name Override the main command name.
     */
    getPath(name) {
        return this._parent
            ? this._parent.getPath(name) + " " + this._name
            : name || this._name;
    }
    /** Get arguments definition. E.g: <input-file:string> <output-file:string> */
    getArgsDefinition() {
        return this.argsDefinition;
    }
    /**
     * Get argument by name.
     *
     * @param name Name of the argument.
     */
    getArgument(name) {
        return this.getArguments().find((arg) => arg.name === name);
    }
    /** Get arguments. */
    getArguments() {
        if (!this.args.length && this.argsDefinition) {
            this.args = parseArgumentsDefinition(this.argsDefinition);
        }
        return this.args;
    }
    /** Check if command has arguments. */
    hasArguments() {
        return !!this.argsDefinition;
    }
    /** Get command version. */
    getVersion() {
        return this.getVersionHandler()?.call(this, this);
    }
    /** Get help handler method. */
    getVersionHandler() {
        return this.ver ?? this._parent?.getVersionHandler();
    }
    /** Get command description. */
    getDescription() {
        // call description method only once
        return typeof this.desc === "function"
            ? this.desc = this.desc()
            : this.desc;
    }
    /** Get auto generated command usage. */
    getUsage() {
        return this._usage ??
            [this.getArgsDefinition(), this.getRequiredOptionsDefinition()]
                .join(" ")
                .trim();
    }
    getRequiredOptionsDefinition() {
        return this.getOptions()
            .filter((option) => option.required)
            .map((option) => [findFlag(option.flags), option.typeDefinition].filter((v) => v)
            .join(" ")
            .trim())
            .join(" ");
    }
    /** Get short command description. This is the first line of the description. */
    getShortDescription() {
        return getDescription(this.getDescription(), true);
    }
    /** Get original command-line arguments. */
    getRawArgs() {
        return this.rawArgs;
    }
    /** Get all arguments defined after the double dash. */
    getLiteralArgs() {
        return this.literalArgs;
    }
    /** Output generated help without exiting. */
    showVersion() {
        console.log(this.getVersion());
    }
    /** Returns command name, version and meta data. */
    getLongVersion() {
        return `${bold(this.getMainCommand().getName())} ${brightBlue(this.getVersion() ?? "")}` +
            Object.entries(this.getMeta()).map(([k, v]) => `\n${bold(k)} ${brightBlue(v)}`).join("");
    }
    /** Outputs command name, version and meta data. */
    showLongVersion() {
        console.log(this.getLongVersion());
    }
    /** Output generated help without exiting. */
    showHelp(options) {
        console.log(this.getHelp(options));
    }
    /** Get generated help. */
    getHelp(options) {
        this.registerDefaults();
        return this.getHelpHandler().call(this, this, options ?? {});
    }
    /** Get help handler method. */
    getHelpHandler() {
        return this._help ?? this._parent?.getHelpHandler();
    }
    exit(code = 0) {
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
    hasOptions(hidden) {
        return this.getOptions(hidden).length > 0;
    }
    /**
     * Get options.
     *
     * @param hidden Include hidden options.
     */
    getOptions(hidden) {
        return this.getGlobalOptions(hidden).concat(this.getBaseOptions(hidden));
    }
    /**
     * Get base options.
     *
     * @param hidden Include hidden options.
     */
    getBaseOptions(hidden) {
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
    getGlobalOptions(hidden) {
        const helpOption = this.getHelpOption();
        const getGlobals = (cmd, noGlobals, options = [], names = []) => {
            if (cmd.options.length) {
                for (const option of cmd.options) {
                    if (option.global &&
                        !this.options.find((opt) => opt.name === option.name) &&
                        names.indexOf(option.name) === -1 &&
                        (hidden || !option.hidden)) {
                        if (noGlobals && option !== helpOption) {
                            continue;
                        }
                        names.push(option.name);
                        options.push(option);
                    }
                }
            }
            return cmd._parent
                ? getGlobals(cmd._parent, noGlobals || cmd._noGlobals, options, names)
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
    hasOption(name, hidden) {
        return !!this.getOption(name, hidden);
    }
    /**
     * Get option by name.
     *
     * @param name Name of the option. Must be in param-case.
     * @param hidden Include hidden options.
     */
    getOption(name, hidden) {
        return this.getBaseOption(name, hidden) ??
            this.getGlobalOption(name, hidden);
    }
    /**
     * Get base option by name.
     *
     * @param name Name of the option. Must be in param-case.
     * @param hidden Include hidden options.
     */
    getBaseOption(name, hidden) {
        const option = this.options.find((option) => option.name === name || option.aliases?.includes(name));
        return option && (hidden || !option.hidden) ? option : undefined;
    }
    /**
     * Get global option from parent commands by name.
     *
     * @param name Name of the option. Must be in param-case.
     * @param hidden Include hidden options.
     */
    getGlobalOption(name, hidden) {
        const helpOption = this.getHelpOption();
        const getGlobalOption = (parent, noGlobals) => {
            const option = parent.getBaseOption(name, hidden);
            if (!option?.global) {
                return parent._parent && getGlobalOption(parent._parent, noGlobals || parent._noGlobals);
            }
            if (noGlobals && option !== helpOption) {
                return;
            }
            return option;
        };
        return this._parent && getGlobalOption(this._parent, this._noGlobals);
    }
    /**
     * Remove option by name.
     *
     * @param name Name of the option. Must be in param-case.
     */
    removeOption(name) {
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
    hasCommands(hidden) {
        return this.getCommands(hidden).length > 0;
    }
    /**
     * Get commands.
     *
     * @param hidden Include hidden commands.
     */
    getCommands(hidden) {
        return this.getGlobalCommands(hidden).concat(this.getBaseCommands(hidden));
    }
    /**
     * Get base commands.
     *
     * @param hidden Include hidden commands.
     */
    getBaseCommands(hidden) {
        const commands = Array.from(this.commands.values());
        return hidden ? commands : commands.filter((cmd) => !cmd.isHidden);
    }
    /**
     * Get global commands.
     *
     * @param hidden Include hidden commands.
     */
    getGlobalCommands(hidden) {
        const getCommands = (command, noGlobals, commands = [], names = []) => {
            if (command.commands.size) {
                for (const [_, cmd] of command.commands) {
                    if (cmd.isGlobal &&
                        this !== cmd &&
                        !this.commands.has(cmd._name) &&
                        names.indexOf(cmd._name) === -1 &&
                        (hidden || !cmd.isHidden)) {
                        if (noGlobals && cmd?.getName() !== "help") {
                            continue;
                        }
                        names.push(cmd._name);
                        commands.push(cmd);
                    }
                }
            }
            return command._parent
                ? getCommands(command._parent, noGlobals || command._noGlobals, commands, names)
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
    hasCommand(name, hidden) {
        return !!this.getCommand(name, hidden);
    }
    /**
     * Get command by name or alias.
     *
     * @param name Name or alias of the command.
     * @param hidden Include hidden commands.
     */
    getCommand(name, hidden) {
        return this.getBaseCommand(name, hidden) ??
            this.getGlobalCommand(name, hidden);
    }
    /**
     * Get base command by name or alias.
     *
     * @param name Name or alias of the command.
     * @param hidden Include hidden commands.
     */
    getBaseCommand(name, hidden) {
        for (const cmd of this.commands.values()) {
            if (cmd._name === name || cmd.aliases.includes(name)) {
                return (cmd && (hidden || !cmd.isHidden) ? cmd : undefined);
            }
        }
    }
    /**
     * Get global command by name or alias.
     *
     * @param name Name or alias of the command.
     * @param hidden Include hidden commands.
     */
    getGlobalCommand(name, hidden) {
        const getGlobalCommand = (parent, noGlobals) => {
            const cmd = parent.getBaseCommand(name, hidden);
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
            getGlobalCommand(this._parent, this._noGlobals);
    }
    /**
     * Remove sub-command by name or alias.
     *
     * @param name Name or alias of the command.
     */
    removeCommand(name) {
        const command = this.getBaseCommand(name, true);
        if (command) {
            this.commands.delete(command._name);
        }
        return command;
    }
    /** Get types. */
    getTypes() {
        return this.getGlobalTypes().concat(this.getBaseTypes());
    }
    /** Get base types. */
    getBaseTypes() {
        return Array.from(this.types.values());
    }
    /** Get global types. */
    getGlobalTypes() {
        const getTypes = (cmd, types = [], names = []) => {
            if (cmd) {
                if (cmd.types.size) {
                    cmd.types.forEach((type) => {
                        if (type.global &&
                            !this.types.has(type.name) &&
                            names.indexOf(type.name) === -1) {
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
    getType(name) {
        return this.getBaseType(name) ?? this.getGlobalType(name);
    }
    /**
     * Get base type by name.
     *
     * @param name Name of the type.
     */
    getBaseType(name) {
        return this.types.get(name);
    }
    /**
     * Get global type by name.
     *
     * @param name Name of the type.
     */
    getGlobalType(name) {
        if (!this._parent) {
            return;
        }
        const cmd = this._parent.getBaseType(name);
        if (!cmd?.global) {
            return this._parent.getGlobalType(name);
        }
        return cmd;
    }
    /** Get completions. */
    getCompletions() {
        return this.getGlobalCompletions().concat(this.getBaseCompletions());
    }
    /** Get base completions. */
    getBaseCompletions() {
        return Array.from(this.completions.values());
    }
    /** Get global completions. */
    getGlobalCompletions() {
        const getCompletions = (cmd, completions = [], names = []) => {
            if (cmd) {
                if (cmd.completions.size) {
                    cmd.completions.forEach((completion) => {
                        if (completion.global &&
                            !this.completions.has(completion.name) &&
                            names.indexOf(completion.name) === -1) {
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
    getCompletion(name) {
        return this.getBaseCompletion(name) ?? this.getGlobalCompletion(name);
    }
    /**
     * Get base completion by name.
     *
     * @param name Name of the completion.
     */
    getBaseCompletion(name) {
        return this.completions.get(name);
    }
    /**
     * Get global completions by name.
     *
     * @param name Name of the completion.
     */
    getGlobalCompletion(name) {
        if (!this._parent) {
            return;
        }
        const completion = this._parent.getBaseCompletion(name);
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
    hasEnvVars(hidden) {
        return this.getEnvVars(hidden).length > 0;
    }
    /**
     * Get environment variables.
     *
     * @param hidden Include hidden environment variable.
     */
    getEnvVars(hidden) {
        return this.getGlobalEnvVars(hidden).concat(this.getBaseEnvVars(hidden));
    }
    /**
     * Get base environment variables.
     *
     * @param hidden Include hidden environment variable.
     */
    getBaseEnvVars(hidden) {
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
    getGlobalEnvVars(hidden) {
        if (this._noGlobals) {
            return [];
        }
        const getEnvVars = (cmd, envVars = [], names = []) => {
            if (cmd) {
                if (cmd.envVars.length) {
                    cmd.envVars.forEach((envVar) => {
                        if (envVar.global &&
                            !this.envVars.find((env) => env.names[0] === envVar.names[0]) &&
                            names.indexOf(envVar.names[0]) === -1 &&
                            (hidden || !envVar.hidden)) {
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
    hasEnvVar(name, hidden) {
        return !!this.getEnvVar(name, hidden);
    }
    /**
     * Get environment variable by name.
     *
     * @param name Name of the environment variable.
     * @param hidden Include hidden environment variable.
     */
    getEnvVar(name, hidden) {
        return this.getBaseEnvVar(name, hidden) ??
            this.getGlobalEnvVar(name, hidden);
    }
    /**
     * Get base environment variable by name.
     *
     * @param name Name of the environment variable.
     * @param hidden Include hidden environment variable.
     */
    getBaseEnvVar(name, hidden) {
        const envVar = this.envVars.find((env) => env.names.indexOf(name) !== -1);
        return envVar && (hidden || !envVar.hidden) ? envVar : undefined;
    }
    /**
     * Get global environment variable by name.
     *
     * @param name Name of the environment variable.
     * @param hidden Include hidden environment variable.
     */
    getGlobalEnvVar(name, hidden) {
        if (!this._parent || this._noGlobals) {
            return;
        }
        const envVar = this._parent.getBaseEnvVar(name, hidden);
        if (!envVar?.global) {
            return this._parent.getGlobalEnvVar(name, hidden);
        }
        return envVar;
    }
    /** Checks whether the command has examples or not. */
    hasExamples() {
        return this.examples.length > 0;
    }
    /** Get all examples. */
    getExamples() {
        return this.examples;
    }
    /** Checks whether the command has an example with given name or not. */
    hasExample(name) {
        return !!this.getExample(name);
    }
    /** Get example with given name. */
    getExample(name) {
        return this.examples.find((example) => example.name === name);
    }
    getHelpOption() {
        return this._helpOption ?? this._parent?.getHelpOption();
    }
}
function findFlag(flags) {
    for (const flag of flags) {
        if (flag.startsWith("--")) {
            return flag;
        }
    }
    return flags[0];
}
