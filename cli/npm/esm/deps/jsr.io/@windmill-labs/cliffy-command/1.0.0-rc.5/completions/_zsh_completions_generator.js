import { MissingCommandNameCompletionsError } from "../_errors.js";
import { getDescription } from "../_utils.js";
import { FileType } from "../types/file.js";
/** Generates zsh completions script. */
export class ZshCompletionsGenerator {
    /** Generates zsh completions script for given command. */
    static generate(name, cmd) {
        if (!name || name === "COMMAND") {
            throw new MissingCommandNameCompletionsError("zsh");
        }
        return new ZshCompletionsGenerator(name, cmd).generate();
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
        Object.defineProperty(this, "actions", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: new Map()
        });
    }
    /** Generates zsh completions code. */
    generate() {
        const path = this.cmd.getPath(this.name);
        const version = this.cmd.getVersion()
            ? ` v${this.cmd.getVersion()}`
            : "";
        return `#compdef ${this.name}

# zsh completion support for ${path}${version}

autoload -U is-at-least

# shellcheck disable=SC2154
(( $+functions[__${replaceSpecialChars(this.name)}_complete] )) ||
function __${replaceSpecialChars(this.name)}_complete {
  local name="$1"; shift
  local action="$1"; shift
  integer ret=1
  local -a values
  local expl lines
  _tags "$name"
  while _tags; do
    if _requested "$name"; then
      # shellcheck disable=SC2034
      lines="$(${this.name} completions complete "\${action}" "\${@}")"
      values=("\${(ps:\\n:)lines}")
      if (( \${#values[@]} )); then
        while _next_label "$name" expl "$action"; do
          compadd -S '' "\${expl[@]}" "\${values[@]}"
        done
      fi
    fi
  done
}

${this.generateCompletions(this.name, this.cmd).trim()}

# shellcheck disable=SC2154
if [ "\${funcstack[1]}" = "_${this.name}" ]; then
  _${replaceSpecialChars(this.name)} "\${@}"
else
  compdef _${replaceSpecialChars(path)} ${path};
fi`;
    }
    /** Generates zsh completions method for given command and child commands. */
    generateCompletions(name, command, path = "") {
        if (!command.hasCommands(false) && !command.hasOptions(false) &&
            !command.hasArguments()) {
            return "";
        }
        path = (path ? path + " " : "") + name;
        return `# shellcheck disable=SC2154
` +
            (command.getParent()
                ? `(( $+functions[_${replaceSpecialChars(path)}] )) || `
                : "") +
            `_${replaceSpecialChars(path)}() {` +
            (!command.getParent()
                ? `
  local state`
                : "") +
            this.generateCommandCompletions(command, path) +
            this.generateSubCommandCompletions(command, path) +
            this.generateArgumentCompletions(command, path) +
            this.generateActions(command) +
            `\n}\n\n` +
            command.getCommands(false)
                .filter((subCommand) => subCommand !== command)
                .map((subCommand) => this.generateCompletions(subCommand.getName(), subCommand, path))
                .join("");
    }
    generateCommandCompletions(command, path) {
        const commands = command.getCommands(false);
        let completions = commands
            .map((subCommand) => `'${subCommand.getName().replace(/:/g, "\\:")}:${subCommand.getShortDescription()
            // escape single quotes
            .replace(/'/g, "'\"'\"'")}'`)
            .join("\n      ");
        if (completions) {
            completions = `
    local -a commands
    # shellcheck disable=SC2034
    commands=(
      ${completions}
    )
    _describe 'command' commands`;
        }
        // only complete first argument, rest arguments are completed with _arguments.
        if (command.hasArguments()) {
            const arg = command.getArguments()[0];
            const type = command.getType(arg.type);
            let action;
            if (type && type.handler instanceof FileType) {
                const fileCompletions = this.getFileCompletions(type);
                completions += `\n    ${fileCompletions}`;
            }
            else {
                const completionsPath = path.split(" ").slice(1).join(" ");
                action = this.addAction(arg, completionsPath);
                if (action && command.getCompletion(arg.action)) {
                    completions += `\n    __${replaceSpecialChars(this.name)}_complete ${action.arg.name} ${action.arg.action} ${action.cmd}`;
                }
            }
        }
        if (command.hasArguments() || command.hasCommands(false)) {
            completions = `\n\n  function _commands() {${completions}\n  }`;
        }
        return completions;
    }
    generateSubCommandCompletions(command, path) {
        if (command.hasCommands(false)) {
            const actions = command
                .getCommands(false)
                .map((command) => {
                const aliases = [command.getName(), ...command.getAliases()]
                    .join("|");
                const action = replaceSpecialChars(path + " " + command.getName());
                return `${aliases}) _${action} ;;`;
            })
                .join("\n      ");
            return `\n
  function _command_args() {
    case "\${words[1]}" in\n      ${actions}\n    esac
  }`;
        }
        return "";
    }
    generateArgumentCompletions(command, path) {
        /* clear actions from previously parsed command. */
        this.actions.clear();
        const options = this.generateOptions(command, path);
        let argIndex = 0;
        // @TODO: add stop early option: -A "-*"
        // http://zsh.sourceforge.net/Doc/Release/Completion-System.html
        let argsCommand = "\n\n  _arguments -w -s -S -C";
        if (command.hasOptions()) {
            argsCommand += ` \\\n    ${options.join(" \\\n    ")}`;
        }
        if (command.hasArguments() || command.hasCommands(false)) {
            const commandArgs = command.getArguments();
            const isVariadic = commandArgs.at(argIndex)?.variadic;
            const selector = isVariadic ? "*" : ++argIndex;
            argsCommand += ` \\\n    '${selector}:command:_commands'`;
            const args = [];
            // first argument is completed together with commands.
            for (const arg of commandArgs.slice(1)) {
                const type = command.getType(arg.type);
                if (type && type.handler instanceof FileType) {
                    const fileCompletions = this.getFileCompletions(type);
                    if (arg.variadic) {
                        argIndex++;
                        for (let i = 0; i < 5; i++) {
                            args.push(`${argIndex + i}${arg.optional ? "::" : ":"}${arg.name}:${fileCompletions}`);
                        }
                    }
                    else {
                        args.push(`${++argIndex}${arg.optional ? "::" : ":"}${arg.name}:${fileCompletions}`);
                    }
                }
                else {
                    const completionsPath = path.split(" ").slice(1).join(" ");
                    const action = this.addAction(arg, completionsPath);
                    args.push(`${++argIndex}${arg.optional ? "::" : ":"}${arg.name}:->${action.name}`);
                }
            }
            argsCommand += args.map((arg) => `\\\n    '${arg}'`).join("");
            if (command.hasCommands(false)) {
                argsCommand += ` \\\n    '*::sub command:->command_args'`;
            }
        }
        return argsCommand;
    }
    generateOptions(command, path) {
        const options = [];
        const cmdArgs = path.split(" ");
        const _baseName = cmdArgs.shift();
        const completionsPath = cmdArgs.join(" ");
        const excludedFlags = command.getOptions(false)
            .map((option) => option.standalone ? option.flags : false)
            .flat()
            .filter((flag) => typeof flag === "string");
        for (const option of command.getOptions(false)) {
            options.push(this.generateOption(command, option, completionsPath, excludedFlags));
        }
        return options;
    }
    generateOption(command, option, completionsPath, excludedOptions) {
        let args = "";
        for (const arg of option.args) {
            const type = command.getType(arg.type);
            const optionalValue = arg.optional ? "::" : ":";
            if (type && type.handler instanceof FileType) {
                const fileCompletions = this.getFileCompletions(type);
                args += `${optionalValue}${arg.name}:${fileCompletions}`;
            }
            else {
                const action = this.addAction(arg, completionsPath);
                args += `${optionalValue}${arg.name}:->${action.name}`;
            }
        }
        const description = getDescription(option.description, true)
            // escape brackets and quotes
            .replace(/\[/g, "\\[")
            .replace(/]/g, "\\]")
            .replace(/"/g, '\\"')
            .replace(/'/g, "'\"'\"'");
        const collect = option.collect ? "*" : "";
        const equalsSign = option.equalsSign ? "=" : "";
        const flags = option.flags.map((flag) => `${flag}${equalsSign}`);
        let result = "";
        if (option.standalone) {
            result += "'(- *)'";
        }
        else {
            const excludedFlags = [...excludedOptions];
            if (option.conflicts?.length) {
                excludedFlags.push(...option.conflicts.map((opt) => "--" + opt.replace(/^--/, "")));
            }
            if (!option.collect) {
                excludedFlags.push(...option.flags);
            }
            if (excludedFlags.length) {
                result += `'(${excludedFlags.join(" ")})'`;
            }
        }
        if (collect || flags.length > 1) {
            result += `{${collect}${flags.join(",")}}`;
        }
        else {
            result += `${flags.join(",")}`;
        }
        return `${result}'[${description}]${args}'`;
    }
    getFileCompletions(type) {
        if (!(type.handler instanceof FileType)) {
            return "";
        }
        return "_files";
        // const fileOpts = type.handler.getOptions();
        // let fileCompletions = "_files";
        // if (fileOpts.dirsOnly) {
        //   fileCompletions += " -/";
        // }
        // if (fileOpts.pattern) {
        //   fileCompletions += ' -g "' + fileOpts.pattern + '"';
        // }
        // if (fileOpts.ignore) {
        //   fileCompletions += " -F " + fileOpts.ignore;
        // }
        // return fileCompletions;
    }
    addAction(arg, cmd) {
        const action = `${arg.name}-${arg.action}`;
        if (!this.actions.has(action)) {
            this.actions.set(action, {
                arg: arg,
                label: `${arg.name}: ${arg.action}`,
                name: action,
                cmd,
            });
        }
        return this.actions.get(action);
    }
    generateActions(command) {
        let actions = [];
        if (this.actions.size) {
            actions = Array
                .from(this.actions)
                .map(([name, action]) => `${name}) __${replaceSpecialChars(this.name)}_complete ${action.arg.name} ${action.arg.action} ${action.cmd} ;;`);
        }
        if (command.hasCommands(false)) {
            actions.unshift(`command_args) _command_args ;;`);
        }
        if (actions.length) {
            return `\n\n  case "$state" in\n    ${actions.join("\n    ")}\n  esac`;
        }
        return "";
    }
}
function replaceSpecialChars(str) {
    return str.replace(/[^a-zA-Z0-9]/g, "_");
}
