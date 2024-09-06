import type { Command } from "../command.js";
import { StringType } from "./string.js";

/** String type with auto completion of sibling command names. */
export class CommandType extends StringType {
  /** Complete sub-command names of global parent command. */
  public complete(_cmd: Command, parent?: Command): string[] {
    return parent?.getCommands(false)
      .map((cmd: Command) => cmd.getName()) || [];
  }
}
