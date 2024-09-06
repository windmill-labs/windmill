import { MissingCommandNameCompletionsError } from "../_errors.js";
import { FileType } from "../types/file.js";
/** Generates bash completions script. */
export class BashCompletionsGenerator {
    /** Generates bash completions script for given command. */
    static generate(name, cmd) {
        if (!name || name === "COMMAND") {
            throw new MissingCommandNameCompletionsError("bash");
        }
        return new BashCompletionsGenerator(name, cmd).generate();
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
    /** Generates bash completions code. */
    generate() {
        const path = this.cmd.getPath(this.name);
        const version = this.cmd.getVersion()
            ? ` v${this.cmd.getVersion()}`
            : "";
        return `#!/usr/bin/env bash
# bash completion support for ${path}${version}

_${replaceSpecialChars(path)}() {
  local word cur prev listFiles
  local -a opts
  COMPREPLY=()
  cur="\${COMP_WORDS[COMP_CWORD]}"
  prev="\${COMP_WORDS[COMP_CWORD-1]}"
  cmd="_"
  opts=()
  listFiles=0

  _${replaceSpecialChars(this.name)}_complete() {
    local action="$1"; shift
    mapfile -t values < <( ${this.name} completions complete "\${action}" "\${@}" )
    for i in "\${values[@]}"; do
      opts+=("$i")
    done
  }

  _${replaceSpecialChars(this.name)}_expand() {
    [ "$cur" != "\${cur%\\\\}" ] && cur="$cur\\\\"
  
    # expand ~username type directory specifications
    if [[ "$cur" == \\~*/* ]]; then
      # shellcheck disable=SC2086
      eval cur=$cur
      
    elif [[ "$cur" == \\~* ]]; then
      cur=\${cur#\\~}
      # shellcheck disable=SC2086,SC2207
      COMPREPLY=( $( compgen -P '~' -u $cur ) )
      return \${#COMPREPLY[@]}
    fi
  }

  # shellcheck disable=SC2120
  _${replaceSpecialChars(this.name)}_file_dir() {
    listFiles=1
    local IFS=$'\\t\\n' xspec #glob
    _${replaceSpecialChars(this.name)}_expand || return 0
  
    if [ "\${1:-}" = -d ]; then
      # shellcheck disable=SC2206,SC2207,SC2086
      COMPREPLY=( \${COMPREPLY[@]:-} $( compgen -d -- $cur ) )
      #eval "$glob"    # restore glob setting.
      return 0
    fi
  
    xspec=\${1:+"!*.$1"}	# set only if glob passed in as $1
    # shellcheck disable=SC2206,SC2207
    COMPREPLY=( \${COMPREPLY[@]:-} $( compgen -f -X "$xspec" -- "$cur" ) \
          $( compgen -d -- "$cur" ) )
  }

  ${this.generateCompletions(this.name, this.cmd).trim()}

  for word in "\${COMP_WORDS[@]}"; do
    case "\${word}" in
      -*) ;;
      *)
        cmd_tmp="\${cmd}_\${word//[^[:alnum:]]/_}"
        if type "\${cmd_tmp}" &>/dev/null; then
          cmd="\${cmd_tmp}"
        fi
    esac
  done

  \${cmd}

  if [[ listFiles -eq 1 ]]; then
    return 0
  fi

  if [[ \${#opts[@]} -eq 0 ]]; then
    # shellcheck disable=SC2207
    COMPREPLY=($(compgen -f "\${cur}"))
    return 0
  fi

  local values
  values="$( printf "\\n%s" "\${opts[@]}" )"
  local IFS=$'\\n'
  # shellcheck disable=SC2207
  local result=($(compgen -W "\${values[@]}" -- "\${cur}"))
  if [[ \${#result[@]} -eq 0 ]]; then
    # shellcheck disable=SC2207
    COMPREPLY=($(compgen -f "\${cur}"))
  else
    # shellcheck disable=SC2207
    COMPREPLY=($(printf '%q\\n' "\${result[@]}"))
  fi

  return 0
}

complete -F _${replaceSpecialChars(path)} -o bashdefault -o default ${path}`;
    }
    /** Generates bash completions method for given command and child commands. */
    generateCompletions(name, command, path = "", index = 1) {
        path = (path ? path + " " : "") + name;
        const commandCompletions = this.generateCommandCompletions(command, path, index);
        const childCommandCompletions = command.getCommands(false)
            .filter((subCommand) => subCommand !== command)
            .map((subCommand) => this.generateCompletions(subCommand.getName(), subCommand, path, index + 1))
            .join("");
        return `${commandCompletions}

${childCommandCompletions}`;
    }
    generateCommandCompletions(command, path, index) {
        const flags = this.getFlags(command);
        const childCommandNames = command.getCommands(false)
            .map((childCommand) => childCommand.getName());
        const completionsPath = ~path.indexOf(" ")
            ? " " + path.split(" ").slice(1).join(" ")
            : "";
        const optionArguments = this.generateOptionArguments(command, completionsPath);
        const completionsCmd = this.generateCommandCompletionsCommand(command, completionsPath);
        return `  __${replaceSpecialChars(path)}() {
    opts=(${[...flags, ...childCommandNames].join(" ")})
    ${completionsCmd}
    if [[ \${cur} == -* || \${COMP_CWORD} -eq ${index} ]] ; then
      return 0
    fi
    ${optionArguments}
  }`;
    }
    getFlags(command) {
        return command.getOptions(false)
            .map((option) => option.flags)
            .flat();
    }
    generateOptionArguments(command, completionsPath) {
        let opts = "";
        const options = command.getOptions(false);
        if (options.length) {
            opts += 'case "${prev}" in';
            for (const option of options) {
                const flags = option.flags
                    .map((flag) => flag.trim())
                    .join("|");
                const completionsCmd = this.generateOptionCompletionsCommand(command, option.args, completionsPath, { standalone: option.standalone });
                opts += `\n      ${flags}) ${completionsCmd} ;;`;
            }
            opts += "\n    esac";
        }
        return opts;
    }
    generateCommandCompletionsCommand(command, path) {
        const args = command.getArguments();
        if (args.length) {
            const type = command.getType(args[0].type);
            if (type && type.handler instanceof FileType) {
                return `_${replaceSpecialChars(this.name)}_file_dir`;
            }
            // @TODO: add support for multiple arguments
            return `_${replaceSpecialChars(this.name)}_complete ${args[0].action}${path}`;
        }
        return "";
    }
    generateOptionCompletionsCommand(command, args, path, opts) {
        if (args.length) {
            const type = command.getType(args[0].type);
            if (type && type.handler instanceof FileType) {
                return `opts=(); _${replaceSpecialChars(this.name)}_file_dir`;
            }
            // @TODO: add support for multiple arguments
            return `opts=(); _${replaceSpecialChars(this.name)}_complete ${args[0].action}${path}`;
        }
        if (opts?.standalone) {
            return "opts=()";
        }
        return "";
    }
}
function replaceSpecialChars(str) {
    return str.replace(/[^a-zA-Z0-9]/g, "_");
}
