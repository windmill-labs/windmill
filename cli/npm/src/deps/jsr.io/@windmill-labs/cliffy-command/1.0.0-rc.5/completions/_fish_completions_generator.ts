import { MissingCommandNameCompletionsError } from "../_errors.js";
import { getDescription } from "../_utils.js";
import type { Command } from "../command.js";
import type { Argument, Option } from "../types.js";
import { FileType } from "../types/file.js";

/** Generates fish completions script. */
interface CompleteOptions {
  description?: string;
  shortOption?: string;
  longOption?: string;
  required?: boolean;
  standalone?: boolean;
  arguments?: string;
}

/** Fish completions generator. */
export class FishCompletionsGenerator {
  /** Generates fish completions script for given command. */
  public static generate(name: string, cmd: Command) {
    if (!name || name === "COMMAND") {
      throw new MissingCommandNameCompletionsError("fish");
    }
    return new FishCompletionsGenerator(name, cmd).generate();
  }

  private constructor(
    protected name: string,
    protected cmd: Command,
  ) {}

  /** Generates fish completions script. */
  private generate(): string {
    const path = this.cmd.getPath(this.name);
    const version: string | undefined = this.cmd.getVersion()
      ? ` v${this.cmd.getVersion()}`
      : "";

    return `#!/usr/bin/env fish
# fish completion support for ${path}${version}

function __fish_${replaceSpecialChars(this.name)}_using_command
  set -l cmds ${getCommandFnNames(this.name, this.cmd).join(" ")}
  set -l words (commandline -opc)
  set -l cmd "_"
  for word in $words
    switch $word
      case '-*'
        continue
      case '*'
        set word (string replace -r -a '\\W' '_' $word)
        set -l cmd_tmp $cmd"_$word"
        if contains $cmd_tmp $cmds
          set cmd $cmd_tmp
        end
    end
  end
  if test "$cmd" = "$argv[1]"
    return 0
  end
  return 1
end

${this.generateCompletions(this.name, this.cmd).trim()}`;
  }

  private generateCompletions(name: string, command: Command): string {
    const parent: Command | undefined = command.getParent();
    let result = ``;

    if (parent) {
      // command
      result += "\n" + this.complete(parent, {
        description: command.getShortDescription(),
        arguments: name,
      });
    }

    // arguments
    const commandArgs = command.getArguments();
    if (commandArgs.length) {
      result += "\n" + this.complete(command, {
        arguments: commandArgs.length
          ? this.getCompletionCommand(command, commandArgs[0])
          : undefined,
      });
    }

    // options
    for (const option of command.getOptions(false)) {
      result += "\n" + this.completeOption(command, option);
    }

    for (const subCommand of command.getCommands(false)) {
      result += this.generateCompletions(subCommand.getName(), subCommand);
    }

    return result;
  }

  private completeOption(command: Command, option: Option) {
    const shortOption: string | undefined = option.flags
      .find((flag) => flag.length === 2)
      ?.replace(/^(-)+/, "");
    const longOption: string | undefined = option.flags
      .find((flag) => flag.length > 2)
      ?.replace(/^(-)+/, "");

    return this.complete(command, {
      description: getDescription(option.description),
      shortOption: shortOption,
      longOption: longOption,
      // required: option.requiredValue,
      required: true,
      standalone: option.standalone,
      arguments: option.args.length
        ? this.getCompletionCommand(command, option.args[0])
        : undefined,
    });
  }

  private complete(command: Command, options: CompleteOptions) {
    const cmd = ["complete"];
    cmd.push("-c", this.name);
    cmd.push(
      "-n",
      `'__fish_${replaceSpecialChars(this.name)}_using_command __${
        replaceSpecialChars(command.getPath(this.name))
      }'`,
    );
    options.shortOption && cmd.push("-s", options.shortOption);
    options.longOption && cmd.push("-l", options.longOption);
    options.standalone && cmd.push("-x");
    cmd.push("-k");
    cmd.push("-f");

    if (options.arguments) {
      options.required && cmd.push("-r");
      cmd.push("-a", options.arguments);
    }

    if (options.description) {
      const description: string = getDescription(options.description, true)
        // escape single quotes
        .replace(/'/g, "\\'");

      cmd.push("-d", `'${description}'`);
    }

    return cmd.join(" ");
  }

  private getCompletionCommand(cmd: Command, arg: Argument): string {
    const type = cmd.getType(arg.type);
    if (type && type.handler instanceof FileType) {
      return `'(__fish_complete_path)'`;
    }
    return `'(${this.name} completions complete ${
      arg.action + " " + getCompletionsPath(cmd)
    })'`;
  }
}

function getCommandFnNames(
  name: string,
  cmd: Command,
  cmds: Array<string> = [],
): Array<string> {
  cmds.push(`__${replaceSpecialChars(cmd.getPath(name))}`);
  cmd.getCommands(false).forEach((command) => {
    getCommandFnNames(name, command, cmds);
  });
  return cmds;
}

function getCompletionsPath(command: Command): string {
  return command.getPath()
    .split(" ")
    .slice(1)
    .join(" ");
}

function replaceSpecialChars(str: string): string {
  return str.replace(/[^a-zA-Z0-9]/g, "_");
}
