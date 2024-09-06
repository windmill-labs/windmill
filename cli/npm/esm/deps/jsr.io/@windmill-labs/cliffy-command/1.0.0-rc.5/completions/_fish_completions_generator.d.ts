import type { Command } from "../command.js";
/** Fish completions generator. */
export declare class FishCompletionsGenerator {
    protected name: string;
    protected cmd: Command;
    /** Generates fish completions script for given command. */
    static generate(name: string, cmd: Command): string;
    private constructor();
    /** Generates fish completions script. */
    private generate;
    private generateCompletions;
    private completeOption;
    private complete;
    private getCompletionCommand;
}
//# sourceMappingURL=_fish_completions_generator.d.ts.map