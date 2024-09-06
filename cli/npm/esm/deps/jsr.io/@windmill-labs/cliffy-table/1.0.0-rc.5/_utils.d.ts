/**
 * Get next words from the beginning of [content] until all words have a length lower or equal then [length].
 *
 * @param length    Max length of all words.
 * @param content   The text content.
 */
import { type CellType } from "./cell.js";
/**
 * Get longest cell from given row index.
 */
export declare function longest(index: number, rows: Array<Array<CellType>>, maxWidth?: number): number;
export declare const strLength: (str: string) => number;
/** Regex `source` to match any relevant ANSI code. */
export declare const ansiRegexSource: string;
/**
 * Get unclosed ANSI runs in a string.
 *
 * @param text - A string segment possibly containing unclosed ANSI runs.
 */
export declare function getUnclosedAnsiRuns(text: string): {
    /** The suffix to be appended to the text to close all unclosed runs. */
    currentSuffix: string;
    /**
     * The prefix to be appended to the next segment to continue unclosed
     * runs if the input text forms the first segment of a multi-line string.
     */
    nextPrefix: string;
};
//# sourceMappingURL=_utils.d.ts.map