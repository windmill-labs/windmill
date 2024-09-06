import type { Command } from "./command.js";
import type { Argument } from "./types.js";
export declare function getFlag(name: string): string;
export declare function didYouMean(message: string, type: string, types: Array<string>): string;
export declare function didYouMeanCommand(command: string, commands: Array<Command>, excludes?: Array<string>): string;
interface SplitArgumentsResult {
    flags: string[];
    typeDefinition: string;
    equalsSign: boolean;
}
/**
 * Split options and arguments.
 * @param args Arguments definition: `--color, -c <color1:string> <color2:string>`
 *
 * For example: `-c, --color <color1:string> <color2:string>`
 *
 * Will result in:
 * ```json
 * {
 *   flags: [ "-c", "--color" ],
 *   typeDefinition: "<color1:string> <color2:string>",
 * }
 * ```
 */
export declare function splitArguments(args: string): SplitArgumentsResult;
/**
 * Parse arguments string.
 * @param argsDefinition Arguments definition: `<color1:string> <color2:string>`
 */
export declare function parseArgumentsDefinition<T extends boolean>(argsDefinition: string, validate: boolean, all: true): Array<Argument | string>;
export declare function parseArgumentsDefinition<T extends boolean>(argsDefinition: string, validate?: boolean, all?: false): Array<Argument>;
export declare function dedent(str: string): string;
export declare function getDescription(description: string, short?: boolean): string;
/** Convert underscore case string to camel case. */
export declare function underscoreToCamelCase(str: string): string;
export {};
//# sourceMappingURL=_utils.d.ts.map