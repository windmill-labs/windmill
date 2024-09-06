import { ansiRegexSource, strLength } from "./_utils.js";

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
export function consumeWords(length: number, content: string): string {
  let consumed = "";
  const words: Array<string> = content.split("\n")[0]?.split(/ /g);

  for (let i = 0; i < words.length; i++) {
    const word: string = words[i];

    // consume minimum one word
    if (consumed) {
      const nextLength = strLength(word);
      const consumedLength = strLength(consumed);
      if (consumedLength + nextLength >= length) {
        break;
      }
    }

    consumed += (i > 0 ? " " : "") + word;
  }

  return consumed;
}

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
export function consumeChars(length: number, content: string): string {
  let consumed = "";
  const chars = [
    ...content.split("\n")[0].matchAll(
      new RegExp(`(?:${ansiRegexSource})+|.`, "gu"),
    ),
  ]
    .map(([match]) => match);

  for (const char of chars) {
    // consume minimum one char
    if (consumed) {
      const nextLength = strLength(char);
      const consumedLength = strLength(consumed);
      if (consumedLength + nextLength > length) {
        break;
      }
    }

    consumed += char;
  }

  return consumed;
}
