var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var _ZshCompletionsCommand_cmd;
import { Command } from "../command.js";
import { dim, italic } from "../../../../@std/fmt/0.225.6/colors.js";
import { ZshCompletionsGenerator } from "./_zsh_completions_generator.js";
/** Generates zsh completions script. */
export class ZshCompletionsCommand extends Command {
    constructor(cmd) {
        super();
        _ZshCompletionsCommand_cmd.set(this, void 0);
        __classPrivateFieldSet(this, _ZshCompletionsCommand_cmd, cmd, "f");
        return this
            .description(() => {
            const baseCmd = __classPrivateFieldGet(this, _ZshCompletionsCommand_cmd, "f") || this.getMainCommand();
            return `Generate shell completions for zsh.

To enable zsh completions for this program add following line to your ${dim(italic("~/.zshrc"))}:

    ${dim(italic(`source <(${baseCmd.getPath()} completions zsh)`))}`;
        })
            .noGlobals()
            .option("-n, --name <command-name>", "The name of the main command.", { default: () => this.getMainCommand().getName() })
            .action(({ name }) => {
            const baseCmd = __classPrivateFieldGet(this, _ZshCompletionsCommand_cmd, "f") || this.getMainCommand();
            console.log(ZshCompletionsGenerator.generate(name, baseCmd));
        });
    }
}
_ZshCompletionsCommand_cmd = new WeakMap();
