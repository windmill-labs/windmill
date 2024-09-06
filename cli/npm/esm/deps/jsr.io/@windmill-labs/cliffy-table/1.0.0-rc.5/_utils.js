/**
 * Get next words from the beginning of [content] until all words have a length lower or equal then [length].
 *
 * @param length    Max length of all words.
 * @param content   The text content.
 */
import { Cell } from "./cell.js";
import { consumeWords } from "./consume_words.js";
import { stripAnsiCode } from "../../../@std/fmt/0.225.6/colors.js";
import { unicodeWidth } from "../../../@std/cli/1.0.0-rc.2/unicode_width.js";
/**
 * Get longest cell from given row index.
 */
export function longest(index, rows, maxWidth) {
    const cellLengths = rows.map((row) => {
        const cell = row[index];
        const cellValue = cell instanceof Cell && cell.getColSpan() > 1
            ? ""
            : cell?.toString() || "";
        return cellValue
            .split("\n")
            .map((line) => {
            const str = typeof maxWidth === "undefined"
                ? line
                : consumeWords(maxWidth, line);
            return strLength(str) || 0;
        });
    }).flat();
    return Math.max(...cellLengths);
}
export const strLength = (str) => {
    return unicodeWidth(stripAnsiCode(str));
};
/** Regex `source` to match any relevant ANSI code. */
export const ansiRegexSource = 
// deno-lint-ignore no-control-regex
/\x1b\[(?:(?<_0>0)|(?<_22>1|2|22)|(?<_23>3|23)|(?<_24>4|24)|(?<_27>7|27)|(?<_28>8|28)|(?<_29>9|29)|(?<_39>30|31|32|33|34|35|36|37|38;2;\d+;\d+;\d+|38;5;\d+|39|90|91|92|93|94|95|96|97)|(?<_49>40|41|42|43|44|45|46|47|48;2;\d+;\d+;\d+|48;5;\d+|49|100|101|102|103|104|105|106|107))m/
    .source;
/**
 * Get unclosed ANSI runs in a string.
 *
 * @param text - A string segment possibly containing unclosed ANSI runs.
 */
export function getUnclosedAnsiRuns(text) {
    const tokens = [];
    for (const { groups } of text.matchAll(new RegExp(ansiRegexSource, "g"))) {
        const [_kind, content] = Object.entries(groups).find(([_, val]) => val);
        tokens.push({ kind: _kind.slice(1), content });
    }
    let unclosed = [];
    for (const token of tokens) {
        // Subsequent ANSI codes of a given kind automatically "close" previous
        // codes of the same kind, so we remove the previous ones.
        // E.g. in the string `${bg_red} A ${bg_yellow} B ${close_bg} C`, "B" only
        // has a single background color (yellow), and "C" has no background color.
        unclosed = [...unclosed.filter((y) => y.kind !== token.kind), token];
    }
    unclosed = unclosed.filter(({ content, kind }) => content !== kind);
    const currentSuffix = unclosed
        .map(({ kind }) => `\x1b[${kind}m`).reverse().join("");
    const nextPrefix = unclosed.map(({ content }) => `\x1b[${content}m`).join("");
    return {
        /** The suffix to be appended to the text to close all unclosed runs. */
        currentSuffix,
        /**
         * The prefix to be appended to the next segment to continue unclosed
         * runs if the input text forms the first segment of a multi-line string.
         */
        nextPrefix,
    };
}
