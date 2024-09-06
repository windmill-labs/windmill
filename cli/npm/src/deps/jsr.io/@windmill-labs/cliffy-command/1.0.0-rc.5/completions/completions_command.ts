import { dim, italic } from "../../../../@std/fmt/0.225.6/colors.js";
import { Command } from "../command.js";
import { BashCompletionsCommand } from "./bash.js";
import { CompleteCommand } from "./complete.js";
import { FishCompletionsCommand } from "./fish.js";
import { ZshCompletionsCommand } from "./zsh.js";

/** Generates shell completion scripts for various shells. */
export class CompletionsCommand
  extends Command<void, void, void, [], { name: string }> {
  #cmd?: Command;

  public constructor(cmd?: Command) {
    super();
    this.#cmd = cmd;
    return this
      .description(() => {
        const baseCmd = this.#cmd || this.getMainCommand();
        return `Generate shell completions.

To enable shell completions for this program add the following line to your ${
          dim(italic("~/.bashrc"))
        } or similar:

    ${dim(italic(`source <(${baseCmd.getPath()} completions [shell])`))}

    For more information run ${
          dim(italic(`${baseCmd.getPath()} completions [shell] --help`))
        }
`;
      })
      .noGlobals()
      .action(() => this.showHelp())
      .command("bash", new BashCompletionsCommand(this.#cmd))
      .command("fish", new FishCompletionsCommand(this.#cmd))
      .command("zsh", new ZshCompletionsCommand(this.#cmd))
      .command("complete", new CompleteCommand(this.#cmd))
      .hidden()
      .reset();
  }
}
