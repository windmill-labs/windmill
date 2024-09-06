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
var _CompletionsCommand_cmd;
import { dim, italic } from "../../../../@std/fmt/0.225.6/colors.js";
import { Command } from "../command.js";
import { BashCompletionsCommand } from "./bash.js";
import { CompleteCommand } from "./complete.js";
import { FishCompletionsCommand } from "./fish.js";
import { ZshCompletionsCommand } from "./zsh.js";
/** Generates shell completion scripts for various shells. */
export class CompletionsCommand extends Command {
    constructor(cmd) {
        super();
        _CompletionsCommand_cmd.set(this, void 0);
        __classPrivateFieldSet(this, _CompletionsCommand_cmd, cmd, "f");
        return this
            .description(() => {
            const baseCmd = __classPrivateFieldGet(this, _CompletionsCommand_cmd, "f") || this.getMainCommand();
            return `Generate shell completions.

To enable shell completions for this program add the following line to your ${dim(italic("~/.bashrc"))} or similar:

    ${dim(italic(`source <(${baseCmd.getPath()} completions [shell])`))}

    For more information run ${dim(italic(`${baseCmd.getPath()} completions [shell] --help`))}
`;
        })
            .noGlobals()
            .action(() => this.showHelp())
            .command("bash", new BashCompletionsCommand(__classPrivateFieldGet(this, _CompletionsCommand_cmd, "f")))
            .command("fish", new FishCompletionsCommand(__classPrivateFieldGet(this, _CompletionsCommand_cmd, "f")))
            .command("zsh", new ZshCompletionsCommand(__classPrivateFieldGet(this, _CompletionsCommand_cmd, "f")))
            .command("complete", new CompleteCommand(__classPrivateFieldGet(this, _CompletionsCommand_cmd, "f")))
            .hidden()
            .reset();
    }
}
_CompletionsCommand_cmd = new WeakMap();
