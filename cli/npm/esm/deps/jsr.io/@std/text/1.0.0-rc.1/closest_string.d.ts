/**
 * The the most similar string from an array of strings.
 *
 * @example Usage
 * ```ts
 * import { closestString } from "@std/text/closest-string";
 * import { assertEquals } from "@std/assert/assert-equals";
 *
 * const possibleWords = ["length", "size", "blah", "help"];
 * const suggestion = closestString("hep", possibleWords);
 *
 * assertEquals(suggestion, "help");
 * ```
 *
 * @param givenWord The string to measure distance against
 * @param possibleWords The string-array that will be sorted
 * @param options An options bag containing a `caseSensitive` flag indicating
 * whether the distance should include case. Default is false.
 * @returns A sorted copy of possibleWords
 * @note
 * the ordering of words may change with version-updates
 * e.g. word-distance metric may change (improve)
 * use a named-distance (e.g. levenshteinDistance) to
 * guarantee a particular ordering
 */
export declare function closestString(givenWord: string, possibleWords: string[], options?: {
    caseSensitive?: boolean;
}): string;
//# sourceMappingURL=closest_string.d.ts.map