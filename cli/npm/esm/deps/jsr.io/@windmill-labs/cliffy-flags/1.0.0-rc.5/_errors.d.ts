import type { ArgumentValue, FlagOptions } from "./types.js";
export declare class FlagsError extends Error {
    constructor(message: string);
}
export declare class UnknownRequiredOptionError extends FlagsError {
    constructor(option: string, options: Array<FlagOptions>);
}
export declare class UnknownConflictingOptionError extends FlagsError {
    constructor(option: string, options: Array<FlagOptions>);
}
export declare class UnknownTypeError extends FlagsError {
    constructor(type: string, types: Array<string>);
}
/**
 * A validation error is thrown when the command is wrongly used by the user.
 * For example: If the user passes some invalid options or arguments to the
 * command.
 */
export declare class ValidationError extends FlagsError {
    constructor(message: string);
}
export declare class DuplicateOptionError extends ValidationError {
    constructor(name: string);
}
export declare class InvalidOptionError extends ValidationError {
    constructor(option: string, options: Array<FlagOptions>);
}
export declare class UnknownOptionError extends ValidationError {
    constructor(option: string, options: Array<FlagOptions>);
}
export declare class MissingOptionValueError extends ValidationError {
    constructor(option: string);
}
export declare class InvalidOptionValueError extends ValidationError {
    constructor(option: string, expected: string, value: string);
}
export declare class UnexpectedOptionValueError extends ValidationError {
    constructor(option: string, value: string);
}
export declare class OptionNotCombinableError extends ValidationError {
    constructor(option: string);
}
export declare class ConflictingOptionError extends ValidationError {
    constructor(option: string, conflictingOption: string);
}
export declare class DependingOptionError extends ValidationError {
    constructor(option: string, dependingOption: string);
}
export declare class MissingRequiredOptionError extends ValidationError {
    constructor(option: string);
}
export declare class UnexpectedRequiredArgumentError extends ValidationError {
    constructor(arg: string);
}
export declare class UnexpectedArgumentAfterVariadicArgumentError extends ValidationError {
    constructor(arg: string);
}
export declare class InvalidTypeError extends ValidationError {
    constructor({ label, name, value, type }: ArgumentValue, expected?: Array<string | number | boolean>);
}
//# sourceMappingURL=_errors.d.ts.map