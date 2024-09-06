import { StringType } from "./string.js";
/** String type with auto completion of sibling command names. */
export class CommandType extends StringType {
    /** Complete sub-command names of global parent command. */
    complete(_cmd, parent) {
        return parent?.getCommands(false)
            .map((cmd) => cmd.getName()) || [];
    }
}
