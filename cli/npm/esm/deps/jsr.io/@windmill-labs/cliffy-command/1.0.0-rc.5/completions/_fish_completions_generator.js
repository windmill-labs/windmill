import { MissingCommandNameCompletionsError } from "../_errors.js";
import { getDescription } from "../_utils.js";
import { FileType } from "../types/file.js";
/** Fish completions generator. */
export class FishCompletionsGenerator {
    /** Generates fish completions script for given command. */
    static generate(name, cmd) {
        if (!name || name === "COMMAND") {
            throw new MissingCommandNameCompletionsError("fish");
        }
        return new FishCompletionsGenerator(name, cmd).generate();
    }
    constructor(name, cmd) {
        Object.defineProperty(this, "name", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: name
        });
        Object.defineProperty(this, "cmd", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: cmd
        });
    }
    /** Generates fish completions script. */
    generate() {
        const path = this.cmd.getPath(this.name);
        const version = this.cmd.getVersion()
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
    generateCompletions(name, command) {
        const parent = command.getParent();
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
    completeOption(command, option) {
        const shortOption = option.flags
            .find((flag) => flag.length === 2)
            ?.replace(/^(-)+/, "");
        const longOption = option.flags
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
    complete(command, options) {
        const cmd = ["complete"];
        cmd.push("-c", this.name);
        cmd.push("-n", `'__fish_${replaceSpecialChars(this.name)}_using_command __${replaceSpecialChars(command.getPath(this.name))}'`);
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
            const description = getDescription(options.description, true)
                // escape single quotes
                .replace(/'/g, "\\'");
            cmd.push("-d", `'${description}'`);
        }
        return cmd.join(" ");
    }
    getCompletionCommand(cmd, arg) {
        const type = cmd.getType(arg.type);
        if (type && type.handler instanceof FileType) {
            return `'(__fish_complete_path)'`;
        }
        return `'(${this.name} completions complete ${arg.action + " " + getCompletionsPath(cmd)})'`;
    }
}
function getCommandFnNames(name, cmd, cmds = []) {
    cmds.push(`__${replaceSpecialChars(cmd.getPath(name))}`);
    cmd.getCommands(false).forEach((command) => {
        getCommandFnNames(name, command, cmds);
    });
    return cmds;
}
function getCompletionsPath(command) {
    return command.getPath()
        .split(" ")
        .slice(1)
        .join(" ");
}
function replaceSpecialChars(str) {
    return str.replace(/[^a-zA-Z0-9]/g, "_");
}
