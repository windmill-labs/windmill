/**
 * Functions for dealing with strings.
 * @module
 */

import { chunk as _chunk } from "./array/base.ts";
import _bytes, { ByteArray } from "./bytes.ts";
import { EMOJI_CHAR } from "./string/constants.ts";

const _chars = chars;

/**
 * Compares two strings, returns `-1` if `a < b`, `0` if `a === b` and `1` if `a > b`.
 * 
 * @example
 * ```ts
 * import { compare } from "@ayonli/jsext/string";
 * 
 * console.log(compare("a", "b")); // -1
 * console.log(compare("b", "a")); // 1
 * console.log(compare("a", "a")); // 0
 * ```
 */
export function compare(str1: string, str2: string): -1 | 0 | 1 {
    if (str1 < str2) {
        return -1;
    } else if (str1 > str2) {
        return 1;
    } else {
        return 0;
    }
}

/**
 * Returns a random string restricted by `length` (character-wise).
 * 
 * @param chars Default value: `0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ`.
 * 
 * @example
 * ```ts
 * import { random } from "@ayonli/jsext/string";
 * 
 * console.log(random(8)); // "2n8G3z1A" for example
 * console.log(random(8, "01")); // "10010101" for example
 * ```
 */
export function random(
    length: number,
    chars = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
): string {
    const arr = _chars(chars);
    let str = "";

    while (0 < length--) {
        const i = Math.floor(Math.random() * arr.length);
        str += arr[i];
    }

    return str;
}

/**
 * Counts the occurrence of the sub-string in the string.
 * 
 * @example
 * ```ts
 * import { count } from "@ayonli/jsext/string";
 * 
 * console.log(count("hello world", "o")); // 2
 * console.log(count("hello world", "i")); // 0 
 * ```
 */
export function count(str: string, sub: string): number {
    if (!sub) {
        return str.length + 1;
    } else if (!str) {
        return 0;
    }

    return str.split(sub).length - 1;
}

/**
 * Capitalizes the string, if `all` is true, all words are capitalized, otherwise only
 * the first word will be capitalized.
 * 
 * @example
 * ```ts
 * import { capitalize } from "@ayonli/jsext/string";
 * 
 * console.log(capitalize("hello world")); // Hello world
 * console.log(capitalize("hello world", true)); // Hello World
 * ```
 */
export function capitalize(str: string, all?: boolean): string {
    const regex = all ? /\w+/g : /\w+/;
    return str.replace(regex, (match) => {
        return (match[0] as string).toUpperCase() + match.slice(1).toLowerCase();
    });
}

/**
 * Replaces the spaces between non-empty characters of the string with hyphens (`-`).
 * 
 * @example
 * ```ts
 * import { hyphenate } from "@ayonli/jsext/string";
 * 
 * console.log(hyphenate("hello world")); // hello-world
 * console.log(hyphenate("hello   world")); // hello-world
 * ```
 */
export function hyphenate(str: string): string {
    return str.replace(/(\S)\s+(\S)/g, (_, $1, $2) => $1 + "-" + $2);
}

/**
 * Returns the bytes of the given string.
 * @deprecated use the `bytes` module instead.
 */
export function bytes(str: string): ByteArray {
    return _bytes(str);
}

/**
 * Returns the characters of the string (emojis are supported).
 * 
 * @example
 * ```ts
 * import { chars } from "@ayonli/jsext/string";
 * 
 * console.log(chars("Hello, World!")); // ["H", "e", "l", "l", "o", ",", " ", "W", "o", "r", "l", "d", "!"]
 * console.log(chars("你好，世界！")) // ["你", "好", "，", "世", "界", "！"]
 * console.log(chars("😴😄⛔🎠🚓🚇👨‍👨‍👧‍👧👦🏾")); // ["😴", "😄", "⛔", "🎠", "🚓", "🚇", "👨‍👨‍👧‍👧", "👦🏾"]
 * ```
 */
export function chars(str: string): string[] {
    if (typeof Intl === "object" && typeof (Intl as any).Segmenter === "function") {
        return Array.from(new (Intl as any).Segmenter().segment(str))
            .map((x: any) => x.segment);
    } else {
        return Array.from(str);
    }
}

/**
 * Extracts words (in latin characters) from the string.
 * 
 * @example
 * ```ts
 * import { words } from "@ayonli/jsext/string";
 * 
 * console.log(words("hello world")); // ["hello", "world"]
 * console.log(words("hello, world")); // ["hello", "world"]
 * console.log(words("hello-world")); // ["hello", "world"]
 * console.log(words("hello_world")); // ["hello", "world"]
 * ```
 */
export function words(str: string): string[] {
    const matches = str.match(/\w+/g);
    return matches ? [...matches].map(sub => sub.split("_")).flat() : [];
}

/**
 * Splits the string into lines by `\n` or `\r\n`.
 * 
 * @example
 * ```ts
 * import { lines } from "@ayonli/jsext/string";
 * 
 * console.log(lines("hello\nworld")); // ["hello", "world"]
 * console.log(lines("hello\r\nworld")); // ["hello", "world"]
 * ```
 */
export function lines(str: string): string[] {
    return str.split(/\r?\n/);
}

/**
 * Breaks the string into smaller chunks according to the given length.
 * 
 * @example
 * ```ts
 * import { chunk } from "@ayonli/jsext/string";
 * 
 * console.log(chunk("hello world", 3)); // ["hel", "lo ", "wor", "ld"]
 * ```
 */
export function chunk(str: string, length: number): string[] {
    return _chunk(str, length) as string[];
}

/**
 * Truncates the string to the given length (including the ending `...`).
 * 
 * @example
 * ```ts
 * import { truncate } from "@ayonli/jsext/string";
 * 
 * console.log(truncate("hello world", 8)); // hello...
 * console.log(truncate("hello world", 11)); // hello world
 * ```
 */
export function truncate(str: string, length: number): string {
    if (length <= 0) {
        return "";
    } else if (length >= str.length) {
        return str;
    } else {
        length -= 3;
        return str.slice(0, length) + "...";
    }
}

const _trim = String.prototype.trim;
const _trimEnd = String.prototype.trimEnd;
const _trimStart = String.prototype.trimStart;

/**
 * Removes leading and trailing spaces or custom characters of the string.
 * 
 * @example
 * ```ts
 * import { trim } from "@ayonli/jsext/string";
 * 
 * console.log(trim("  hello world  ")); // "hello world"
 * console.log(trim("  hello world!  ", " !")); // "hello world"
 * ```
 */
export function trim(str: string, chars = ""): string {
    if (!chars) {
        return _trim.call(str);
    } else {
        return trimEnd(trimStart(str, chars), chars);
    }
}

/**
 * Removes trailing spaces or custom characters of the string.
 * 
 * @example
 * ```ts
 * import { trimEnd } from "@ayonli/jsext/string";
 * 
 * console.log(trimEnd("  hello world  ")); // "  hello world"
 * console.log(trimEnd("  hello world!  ", " !")); // "  hello world"
 * ```
 */
export function trimEnd(str: string, chars = ""): string {
    if (!chars) {
        return _trimEnd.call(str);
    } else {
        let i = str.length;
        while (i-- && chars.indexOf(str[i] as string) !== -1) { }
        return str.substring(0, i + 1);
    }
}

/**
 * Removes leading spaces or custom characters of the string.
 * 
 * @example
 * ```ts
 * import { trimStart } from "@ayonli/jsext/string";
 * 
 * console.log(trimStart("  hello world  ")); // "hello world  "
 * console.log(trimStart("  !hello world!  ", " !")); // "hello world!  "
 * ```
 */
export function trimStart(str: string, chars = ""): string {
    if (!chars) {
        return _trimStart.call(str);
    } else {
        let i = 0;
        do { } while (chars.indexOf(str[i] as string) !== -1 && ++i);
        return str.substring(i);
    }
}

/**
 * Removes the given suffix of the string if present.
 * 
 * @example
 * ```ts
 * import { stripEnd } from "@ayonli/jsext/string";
 * 
 * console.log(stripEnd("hello world", "world")); // "hello "
 * console.log(stripEnd("hello world", "hello")); // "hello world"
 * ```
 */
export function stripEnd(str: string, suffix: string): string {
    if (str.endsWith(suffix)) {
        return str.slice(0, -suffix.length);
    }

    return str;
}

/**
 * Removes the given prefix of the string if present.
 * 
 * @example
 * ```ts
 * import { stripStart } from "@ayonli/jsext/string";
 * 
 * console.log(stripStart("hello world", "hello")); // " world"
 * console.log(stripStart("hello world", "hi")); // "hello world"
 * ```
 */
export function stripStart(str: string, prefix: string): string {
    if (str.startsWith(prefix)) {
        return str.slice(prefix.length);
    }

    return str;
}

/**
 * Removes extra indentation from the string.
 * 
 * **NOTE:** This function also removes leading and trailing newlines.
 * 
 * @example
 * ```ts
 * import { dedent } from "@ayonli/jsext/string";
 * 
 * class MyClass {
 *     print() {
 *         console.log(dedent(`
 *             create table student(
 *                 id int primary key,
 *                 name text
 *             )
 *        `));
 *     }
 * }
 * 
 * new MyClass().print();
 * // Output:
 * // create table student(
 * //     id int primary key,
 * //     name text
 * // )
 * ```
 */
export function dedent(str: string): string;
/**
 * This function can also be used as a template literal tag.
 * 
 * @see https://github.com/tc39/proposal-string-dedent
 */
export function dedent(str: TemplateStringsArray, ...values: any[]): string;
export function dedent(str: string | TemplateStringsArray, ...values: any[]): string {
    if (Array.isArray(str)) {
        str = (str as TemplateStringsArray)
            .reduce((acc, cur, i) => acc + cur + (values[i] || ""), "");
    }

    const oldLines = lines(str as string);
    const newLines: string[] = [];
    let indent = "";

    for (const line of oldLines) {
        const match = line.match(/^(\s+)\S+/);
        if (match) {
            if (!indent || match[1]!.length < indent.length) {
                indent = match[1]!;
            }
        }
    }

    for (const line of oldLines) {
        if (line.startsWith(indent)) {
            newLines.push(line.slice(indent.length));
        } else {
            newLines.push(line);
        }
    }

    return newLines.join("\n").trim();
}

/**
 * Returns the byte length of the string.
 * 
 * @example
 * ```ts
 * import { byteLength } from "@ayonli/jsext/string";
 * 
 * console.log(byteLength("hello world")); // 11
 * console.log(byteLength("你好，世界！")); // 18
 * ```
 */
export function byteLength(str: string): number {
    return _bytes(str).byteLength;
}

/** Checks if all characters in the string are within the ASCII range. */
export function isAscii(str: string, printableOnly = false): boolean {
    return printableOnly ? /^[-~]+$/.test(str) : /^[\x00-\x7E]+$/.test(str);
}

/** Checks if all characters in the string are emojis. */
export function isEmoji(str: string): boolean {
    return chars(str).every((char) => EMOJI_CHAR!.test(char));
}
