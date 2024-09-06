import type { Command } from "../command.js";
/** Generates zsh completions script. */
export declare class ZshCompletionsGenerator {
    protected name: string;
    protected cmd: Command;
    private actions;
    /** Generates zsh completions script for given command. */
    static generate(name: string, cmd: Command): string;
    private constructor();
    /** Generates zsh completions code. */
    private generate;
    /** Generates zsh completions method for given command and child commands. */
    private generateCompletions;
    private generateCommandCompletions;
    private generateSubCommandCompletions;
    private generateArgumentCompletions;
    private generateOptions;
    private generateOption;
    private getFileCompletions;
    private addAction;
    private generateActions;
}
//# sourceMappingURL=_zsh_completions_generator.d.ts.map