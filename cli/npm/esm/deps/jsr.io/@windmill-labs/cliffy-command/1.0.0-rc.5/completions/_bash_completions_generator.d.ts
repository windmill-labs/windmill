import type { Command } from "../command.js";
/** Generates bash completions script. */
export declare class BashCompletionsGenerator {
    protected name: string;
    protected cmd: Command;
    /** Generates bash completions script for given command. */
    static generate(name: string, cmd: Command): string;
    private constructor();
    /** Generates bash completions code. */
    private generate;
    /** Generates bash completions method for given command and child commands. */
    private generateCompletions;
    private generateCommandCompletions;
    private getFlags;
    private generateOptionArguments;
    private generateCommandCompletionsCommand;
    private generateOptionCompletionsCommand;
}
//# sourceMappingURL=_bash_completions_generator.d.ts.map