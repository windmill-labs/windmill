import { didYouMeanOption, didYouMeanType, getFlag } from "./_utils.js";
import type { ArgumentValue, FlagOptions } from "./types.js";

export class FlagsError extends Error {
  constructor(message: string) {
    super(message);
    Object.setPrototypeOf(this, FlagsError.prototype);
  }
}

export class UnknownRequiredOptionError extends FlagsError {
  constructor(option: string, options: Array<FlagOptions>) {
    super(
      `Unknown required option "${getFlag(option)}".${
        didYouMeanOption(option, options)
      }`,
    );
    Object.setPrototypeOf(this, UnknownRequiredOptionError.prototype);
  }
}

export class UnknownConflictingOptionError extends FlagsError {
  constructor(option: string, options: Array<FlagOptions>) {
    super(
      `Unknown conflicting option "${getFlag(option)}".${
        didYouMeanOption(option, options)
      }`,
    );
    Object.setPrototypeOf(this, UnknownConflictingOptionError.prototype);
  }
}

export class UnknownTypeError extends FlagsError {
  constructor(type: string, types: Array<string>) {
    super(`Unknown type "${type}".${didYouMeanType(type, types)}`);
    Object.setPrototypeOf(this, UnknownTypeError.prototype);
  }
}

/* Validation errors. */

/**
 * A validation error is thrown when the command is wrongly used by the user.
 * For example: If the user passes some invalid options or arguments to the
 * command.
 */
export class ValidationError extends FlagsError {
  constructor(message: string) {
    super(message);
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

export class DuplicateOptionError extends ValidationError {
  constructor(name: string) {
    super(
      `Option "${
        getFlag(name).replace(/^--no-/, "--")
      }" can only occur once, but was found several times.`,
    );
    Object.setPrototypeOf(this, DuplicateOptionError.prototype);
  }
}

export class InvalidOptionError extends ValidationError {
  constructor(option: string, options: Array<FlagOptions>) {
    super(
      `Invalid option "${getFlag(option)}".${
        didYouMeanOption(option, options)
      }`,
    );
    Object.setPrototypeOf(this, InvalidOptionError.prototype);
  }
}

export class UnknownOptionError extends ValidationError {
  constructor(option: string, options: Array<FlagOptions>) {
    super(
      `Unknown option "${getFlag(option)}".${
        didYouMeanOption(option, options)
      }`,
    );
    Object.setPrototypeOf(this, UnknownOptionError.prototype);
  }
}

export class MissingOptionValueError extends ValidationError {
  constructor(option: string) {
    super(`Missing value for option "${getFlag(option)}".`);
    Object.setPrototypeOf(this, MissingOptionValueError.prototype);
  }
}

export class InvalidOptionValueError extends ValidationError {
  constructor(option: string, expected: string, value: string) {
    super(
      `Option "${
        getFlag(option)
      }" must be of type "${expected}", but got "${value}".`,
    );
    Object.setPrototypeOf(this, InvalidOptionValueError.prototype);
  }
}

export class UnexpectedOptionValueError extends ValidationError {
  constructor(option: string, value: string) {
    super(
      `Option "${getFlag(option)}" doesn't take a value, but got "${value}".`,
    );
    Object.setPrototypeOf(this, InvalidOptionValueError.prototype);
  }
}

export class OptionNotCombinableError extends ValidationError {
  constructor(option: string) {
    super(`Option "${getFlag(option)}" cannot be combined with other options.`);
    Object.setPrototypeOf(this, OptionNotCombinableError.prototype);
  }
}

export class ConflictingOptionError extends ValidationError {
  constructor(option: string, conflictingOption: string) {
    super(
      `Option "${getFlag(option)}" conflicts with option "${
        getFlag(conflictingOption)
      }".`,
    );
    Object.setPrototypeOf(this, ConflictingOptionError.prototype);
  }
}

export class DependingOptionError extends ValidationError {
  constructor(option: string, dependingOption: string) {
    super(
      `Option "${getFlag(option)}" depends on option "${
        getFlag(dependingOption)
      }".`,
    );
    Object.setPrototypeOf(this, DependingOptionError.prototype);
  }
}

export class MissingRequiredOptionError extends ValidationError {
  constructor(option: string) {
    super(`Missing required option "${getFlag(option)}".`);
    Object.setPrototypeOf(this, MissingRequiredOptionError.prototype);
  }
}

export class UnexpectedRequiredArgumentError extends ValidationError {
  constructor(arg: string) {
    super(
      `An required argument cannot follow an optional argument, but "${arg}"  is defined as required.`,
    );
    Object.setPrototypeOf(
      this,
      UnexpectedRequiredArgumentError.prototype,
    );
  }
}

export class UnexpectedArgumentAfterVariadicArgumentError
  extends ValidationError {
  constructor(arg: string) {
    super(`An argument cannot follow an variadic argument, but got "${arg}".`);
    Object.setPrototypeOf(
      this,
      UnexpectedArgumentAfterVariadicArgumentError.prototype,
    );
  }
}

export class InvalidTypeError extends ValidationError {
  constructor(
    { label, name, value, type }: ArgumentValue,
    expected?: Array<string | number | boolean>,
  ) {
    super(
      `${label} "${name}" must be of type "${type}", but got "${value}".` + (
        expected
          ? ` Expected values: ${
            expected.map((value) => `"${value}"`).join(", ")
          }`
          : ""
      ),
    );
    Object.setPrototypeOf(this, MissingOptionValueError.prototype);
  }
}
