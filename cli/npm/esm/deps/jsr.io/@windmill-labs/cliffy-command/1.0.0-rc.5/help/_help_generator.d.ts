import type { Command } from "../command.js";
export interface HelpOptions {
    types?: boolean;
    hints?: boolean;
    colors?: boolean;
    long?: boolean;
}
/** Help text generator. */
export declare class HelpGenerator {
    private cmd;
    private indent;
    private options;
    /** Generate help text for given command. */
    static generate(cmd: Command, options?: HelpOptions): string;
    private constructor();
    private generate;
    private generateHeader;
    private generateMeta;
    private generateDescription;
    private generateOptions;
    private generateOptionGroup;
    private generateCommands;
    private generateEnvironmentVariables;
    private generateExamples;
    private generateHints;
    private label;
}
//# sourceMappingURL=_help_generator.d.ts.map