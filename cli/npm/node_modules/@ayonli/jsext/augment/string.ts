import type { ByteArray } from "../bytes.ts";
import {
    compare,
    random,
    capitalize as _capitalize,
    chunk as _chunk,
    count as _count,
    hyphenate as _hyphenate,
    truncate as _truncate,
    bytes as _bytes,
    chars as _chars,
    words as _words,
    lines as _lines,
    trim as _trim,
    trimEnd as _trimEnd,
    trimStart as _trimStart,
    stripEnd as _stripEnd,
    stripStart as _stripStart,
    dedent as _dedent,
    byteLength as _byteLength,
    isAscii as _isAscii,
    isEmoji as _isEmoji,
} from "../string.ts";

declare global {
    interface StringConstructor {
        /**
         * Compares two strings, returns `-1` if `a < b`, `0` if `a === b` and `1` if `a > b`.
         */
        compare(str1: string, str2: string): -1 | 0 | 1;
        /**
         * Returns a random string restricted by `length` (character-wise).
         * 
         * @param chars Default value: `0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ`.
         */
        random(length: number, chars?: string): string;
        /**
         * A template literal tag tag that removes extra indentation from the string.
         * 
         * @see https://github.com/tc39/proposal-string-dedent
         */
        dedent(strings: TemplateStringsArray, ...values: any[]): string;
    }

    interface String {
        /** Counts the occurrence of the sub-string in the string. */
        count(sub: string): number;
        /**
         * Capitalizes the string, if `all` is true, all words are capitalized, otherwise only
         * the first word will be capitalized.
         */
        capitalize(all?: boolean): string;
        /** Replaces the spaces between non-empty characters of the string with hyphens (`-`). */
        hyphenate(): string;
        /** Returns the bytes of the given string. */
        bytes(): ByteArray;
        /** Returns the characters of the string (emojis are supported). */
        chars(): string[];
        /** Extracts words (in latin characters) from the string. */
        words(): string[];
        /** Splits the string into lines by `\n` or `\r\n`. */
        lines(): string[];
        /** Breaks the string into smaller chunks according to the given length. */
        chunk(length: number): string[];
        /** Truncates the string to the given length (including the ending `...`). */
        truncate(length: number): string;
        /** Removes leading and trailing spaces or custom characters of the string. */
        trim(chars?: string): string;
        /** Removes trailing spaces or custom characters of the string. */
        trimEnd(chars?: string): string;
        /** Removes leading spaces or custom characters of the string. */
        trimStart(chars?: string): string;
        /** Removes the given prefix of the string if present. */
        stripStart(prefix: string): string;
        /** Removes the given suffix of the string if present. */
        stripEnd(suffix: string): string;
        /**
         * Removes extra indentation from the string.
         * 
         * **NOTE:** This function also removes leading and trailing newlines.
         */
        dedent(): string;
        /** Returns the byte length of the string. */
        byteLength(): number;
        /** Checks if all characters in the string are within the ASCII range. */
        isAscii(printableOnly?: boolean): boolean;
        /** Checks if all characters in the string are emojis. */
        isEmoji(): boolean;
    }
}

String.compare = compare;
String.random = random;

if (typeof String.dedent === "undefined") {
    String.dedent = _dedent;
}

String.prototype.count = function count(sub) {
    return _count(String(this), sub);
};

String.prototype.capitalize = function capitalize(all) {
    return _capitalize(String(this), all);
};

String.prototype.hyphenate = function capitalize() {
    return _hyphenate(String(this));
};

String.prototype.bytes = function bytes() {
    return _bytes(String(this));
};

String.prototype.chars = function chars() {
    return _chars(String(this));
};

String.prototype.words = function words() {
    return _words(String(this));
};

String.prototype.lines = function lines() {
    return _lines(String(this));
};

String.prototype.chunk = function chunk(length) {
    return _chunk(String(this), length);
};

String.prototype.truncate = function truncate(length) {
    return _truncate(String(this), length);
};

String.prototype.trim = function trim(chars: string = "") {
    return _trim(String(this), chars);
};

String.prototype.trimEnd = function trimEnd(chars: string = "") {
    return _trimEnd(String(this), chars);
};

String.prototype.trimStart = function trimStart(chars: string = "") {
    return _trimStart(String(this), chars);
};

String.prototype.stripEnd = function stripEnd(suffix: string) {
    return _stripEnd(String(this), suffix);
};

String.prototype.stripStart = function stripStart(prefix: string) {
    return _stripStart(String(this), prefix);
};

if (typeof String.prototype.dedent === "undefined") {
    String.prototype.dedent = function dedent() {
        return _dedent(String(this));
    };
}

String.prototype.byteLength = function byteLength() {
    return _byteLength(String(this));
};

String.prototype.isAscii = function isAscii(printableOnly = false) {
    return _isAscii(String(this), printableOnly);
};

String.prototype.isEmoji = function isEmoji() {
    return _isEmoji(String(this));
};
