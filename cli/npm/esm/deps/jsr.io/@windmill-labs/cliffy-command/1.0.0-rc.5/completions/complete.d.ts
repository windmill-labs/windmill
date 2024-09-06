import { Command } from "../command.js";
/** Execute auto completion method of command and action. */
export declare class CompleteCommand extends Command<void, void, void, [
    action: string,
    ...commandNames: Array<string>
]> {
    constructor(cmd?: Command);
}
//# sourceMappingURL=complete.d.ts.map