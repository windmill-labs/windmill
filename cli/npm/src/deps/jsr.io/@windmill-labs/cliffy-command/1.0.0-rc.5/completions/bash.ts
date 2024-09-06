import { Command } from "../command.js";
import { dim, italic } from "../../../../@std/fmt/0.225.6/colors.js";
import { BashCompletionsGenerator } from "./_bash_completions_generator.js";

/** Generates bash completions script. */
export class BashCompletionsCommand
  extends Command<void, void, { name: string }> {
  readonly #cmd?: Command;

  public constructor(cmd?: Command) {
    super();
    this.#cmd = cmd;
    return this
      .description(() => {
        const baseCmd = this.#cmd || this.getMainCommand();
        return `Generate shell completions for bash.

To enable bash completions for this program add following line to your ${
          dim(italic("~/.bashrc"))
        }:

    ${dim(italic(`source <(${baseCmd.getPath()} completions bash)`))}`;
      })
      .noGlobals()
      .option("-n, --name <command-name>", "The name of the main command.")
      .action(({ name = this.getMainCommand().getName() }) => {
        const baseCmd = this.#cmd || this.getMainCommand();
        console.log(BashCompletionsGenerator.generate(name, baseCmd));
      });
  }
}
