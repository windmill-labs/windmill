/**
 * Calculates the
 * {@link https://en.wikipedia.org/wiki/Levenshtein_distance | Levenshtein distance}
 * between two strings.
 *
 * @example Usage
 * ```ts
 * import { levenshteinDistance } from "@std/text/levenshtein-distance";
 * import { assertEquals } from "@std/assert/assert-equals";
 *
 * assertEquals(levenshteinDistance("aa", "bb"), 2);
 * ```
 * @param str1 The first string.
 * @param str2 The second string.
 * @returns The Levenshtein distance between the two strings.
 */
export declare function levenshteinDistance(str1: string, str2: string): number;
//# sourceMappingURL=levenshtein_distance.d.ts.map