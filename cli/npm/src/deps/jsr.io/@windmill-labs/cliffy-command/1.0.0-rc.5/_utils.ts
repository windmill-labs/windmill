import {
  OptionType,
  UnexpectedArgumentAfterVariadicArgumentError,
  UnexpectedRequiredArgumentError,
} from "../../cliffy-flags/1.0.0-rc.5/mod.js";
import { closestString } from "../../../@std/text/1.0.0-rc.1/closest_string.js";
import type { Command } from "./command.js";
import type { Argument } from "./types.js";

export function getFlag(name: string) {
  if (name.startsWith("-")) {
    return name;
  }
  if (name.length > 1) {
    return `--${name}`;
  }
  return `-${name}`;
}

export function didYouMean(
  message: string,
  type: string,
  types: Array<string>,
): string {
  const match: string | undefined = types.length
    ? closestString(type, types)
    : undefined;
  return match ? `${message} "${match}"?` : "";
}

export function didYouMeanCommand(
  command: string,
  commands: Array<Command>,
  excludes: Array<string> = [],
): string {
  const commandNames = commands
    .map((command) => command.getName())
    .filter((command) => !excludes.includes(command));

  return didYouMean(" Did you mean command", command, commandNames);
}

const ARGUMENT_REGEX = /^[<\[].+[\]>]$/;
const ARGUMENT_DETAILS_REGEX = /[<\[:>\]]/;

interface SplitArgumentsResult {
  flags: string[];
  typeDefinition: string;
  equalsSign: boolean;
}

/**
 * Split options and arguments.
 * @param args Arguments definition: `--color, -c <color1:string> <color2:string>`
 *
 * For example: `-c, --color <color1:string> <color2:string>`
 *
 * Will result in:
 * ```json
 * {
 *   flags: [ "-c", "--color" ],
 *   typeDefinition: "<color1:string> <color2:string>",
 * }
 * ```
 */
export function splitArguments(
  args: string,
): SplitArgumentsResult {
  const parts = args.trim().split(/[, =] */g);
  const typeParts = [];

  while (
    parts[parts.length - 1] &&
    ARGUMENT_REGEX.test(parts[parts.length - 1])
  ) {
    typeParts.unshift(parts.pop());
  }

  const typeDefinition: string = typeParts.join(" ");

  return { flags: parts, typeDefinition, equalsSign: args.includes("=") };
}

/**
 * Parse arguments string.
 * @param argsDefinition Arguments definition: `<color1:string> <color2:string>`
 */
export function parseArgumentsDefinition<T extends boolean>(
  argsDefinition: string,
  validate: boolean,
  all: true,
): Array<Argument | string>;
export function parseArgumentsDefinition<T extends boolean>(
  argsDefinition: string,
  validate?: boolean,
  all?: false,
): Array<Argument>;
export function parseArgumentsDefinition<T extends boolean>(
  argsDefinition: string,
  validate = true,
  all?: T,
): T extends true ? Array<Argument | string> : Array<Argument> {
  const argumentDetails: Array<Argument | string> = [];

  let hasOptional = false;
  let hasVariadic = false;
  const parts: string[] = argsDefinition.split(/ +/);

  for (const arg of parts) {
    if (validate && hasVariadic) {
      throw new UnexpectedArgumentAfterVariadicArgumentError(arg);
    }
    const parts: string[] = arg.split(ARGUMENT_DETAILS_REGEX);

    if (!parts[1]) {
      if (all) {
        argumentDetails.push(parts[0]);
      }
      continue;
    }
    const type: string | undefined = parts[2] || OptionType.STRING;

    const details: Argument = {
      optional: arg[0] === "[",
      name: parts[1],
      action: parts[3] || type,
      variadic: false,
      list: type ? arg.indexOf(type + "[]") !== -1 : false,
      type,
    };

    if (validate && !details.optional && hasOptional) {
      throw new UnexpectedRequiredArgumentError(details.name);
    }

    if (arg[0] === "[") {
      hasOptional = true;
    }

    if (details.name.length > 3) {
      const istVariadicLeft = details.name.slice(0, 3) === "...";
      const istVariadicRight = details.name.slice(-3) === "...";

      hasVariadic = details.variadic = istVariadicLeft || istVariadicRight;

      if (istVariadicLeft) {
        details.name = details.name.slice(3);
      } else if (istVariadicRight) {
        details.name = details.name.slice(0, -3);
      }
    }

    argumentDetails.push(details);
  }

  return argumentDetails as (
    T extends true ? Array<Argument | string> : Array<Argument>
  );
}

export function dedent(str: string): string {
  const lines = str.split(/\r?\n|\r/g);
  let text = "";
  let indent = 0;

  for (const line of lines) {
    if (text || line.trim()) {
      if (!text) {
        text = line.trimStart();
        indent = line.length - text.length;
      } else {
        text += line.slice(indent);
      }
      text += "\n";
    }
  }

  return text.trimEnd();
}

export function getDescription(
  description: string,
  short?: boolean,
): string {
  return short
    ? description.trim().split("\n", 1)[0].trim()
    : dedent(description);
}

/** Convert underscore case string to camel case. */
export function underscoreToCamelCase(str: string): string {
  return str
    .replace(/([a-z])([A-Z])/g, "$1_$2")
    .toLowerCase()
    .replace(
      /_([a-z])/g,
      (g) => g[1].toUpperCase(),
    );
}
