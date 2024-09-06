import type { Command } from "../command.js";
import { StringType } from "./string.js";
/** String type with auto completion of sibling command names. */
export declare class CommandType extends StringType {
    /** Complete sub-command names of global parent command. */
    complete(_cmd: Command, parent?: Command): string[];
}
//# sourceMappingURL=command.d.ts.map