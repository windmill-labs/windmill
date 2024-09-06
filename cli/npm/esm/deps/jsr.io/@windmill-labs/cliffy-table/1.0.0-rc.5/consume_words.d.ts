/**
 * Consumes the maximum amount of words from a string which is not longer than
 * given length. This function returns at least one word.
 *
 * ```ts
 * import { consumeWords } from "./mod.ts";
 *
 * const str = consumeWords(9, "This is an example."); // returns: "This is"
 * ```
 *
 * @param length  The maximum length of the returned string.
 * @param content The content from which the string should be consumed.
 */
export declare function consumeWords(length: number, content: string): string;
/**
 * Consumes the maximum amount of chars from a string which is not longer than
 * given length, ignoring ANSI codes when calculating the length.
 * This function returns at least one char.
 *
 * ```ts
 * import { consumeChars } from "./consume_words.ts";
 *
 * const str = consumeChars(9, "\x1b[31mThis is an example."); // returns: "\x1b[31mThis is a"
 * ```
 *
 * @param length  The maximum length of the returned string.
 * @param content The content from which the string should be consumed.
 */
export declare function consumeChars(length: number, content: string): string;
//# sourceMappingURL=consume_words.d.ts.map