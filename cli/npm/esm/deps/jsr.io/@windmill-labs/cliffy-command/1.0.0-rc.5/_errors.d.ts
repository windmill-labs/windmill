import type { Command } from "./command.js";
import type { EnvVar } from "./types.js";
export declare class CommandError extends Error {
    constructor(message: string);
}
export interface ValidationErrorOptions {
    exitCode?: number;
}
export declare class ValidationError extends CommandError {
    readonly exitCode: number;
    cmd?: Command;
    constructor(message: string, { exitCode }?: ValidationErrorOptions);
}
export declare class DuplicateOptionNameError extends CommandError {
    constructor(optionName: string, commandName: string);
}
export declare class MissingCommandNameCompletionsError extends CommandError {
    constructor(shell: string);
}
export declare class MissingCommandNameError extends CommandError {
    constructor();
}
export declare class DuplicateCommandNameError extends CommandError {
    constructor(name: string);
}
export declare class DuplicateCommandAliasError extends CommandError {
    constructor(alias: string);
}
export declare class CommandNotFoundError extends CommandError {
    constructor(name: string, commands: Array<Command>, excluded?: Array<string>);
}
export declare class DuplicateTypeError extends CommandError {
    constructor(name: string);
}
export declare class DuplicateCompletionError extends CommandError {
    constructor(name: string);
}
export declare class DuplicateExampleError extends CommandError {
    constructor(name: string);
}
export declare class DuplicateEnvVarError extends CommandError {
    constructor(name: string);
}
export declare class MissingRequiredEnvVarError extends ValidationError {
    constructor(envVar: EnvVar);
}
export declare class TooManyEnvVarValuesError extends CommandError {
    constructor(name: string);
}
export declare class UnexpectedOptionalEnvVarValueError extends CommandError {
    constructor(name: string);
}
export declare class UnexpectedVariadicEnvVarValueError extends CommandError {
    constructor(name: string);
}
export declare class DefaultCommandNotFoundError extends CommandError {
    constructor(name: string, commands: Array<Command>);
}
export declare class UnknownCompletionCommandError extends CommandError {
    constructor(name: string, commands: Array<Command>);
}
export declare class UnknownCommandError extends ValidationError {
    constructor(name: string, commands: Array<Command>, excluded?: Array<string>);
}
export declare class NoArgumentsAllowedError extends ValidationError {
    constructor(name: string);
}
export declare class MissingArgumentsError extends ValidationError {
    constructor(names: Array<string>);
}
export declare class MissingArgumentError extends ValidationError {
    constructor(name: string);
}
export declare class TooManyArgumentsError extends ValidationError {
    constructor(args: Array<string>);
}
//# sourceMappingURL=_errors.d.ts.map