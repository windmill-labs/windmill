export declare const LF: "\n";
/** End-of-line character for Windows platforms. */
export declare const CRLF: "\r\n";
/**
 * End-of-line character evaluated for the current platform.
 *
 * @example Usage
 * ```ts no-eval
 * import { EOL } from "@std/fs/eol";
 *
 * EOL; // "\n" on POSIX platforms and "\r\n" on Windows
 * ```
 */
export declare const EOL: "\n" | "\r\n";
/**
 * Returns the detected EOL character(s) detected in the input string. If no EOL
 * character is detected, `null` is returned.
 *
 * @param content The input string to detect EOL characters.
 *
 * @returns The detected EOL character(s) or `null` if no EOL character is detected.
 *
 * @example Usage
 * ```ts no-eval
 * import { detect } from "@std/fs/eol";
 *
 * detect("deno\r\nis not\r\nnode"); // "\r\n"
 * detect("deno\nis not\r\nnode"); // "\r\n"
 * detect("deno\nis not\nnode"); // "\n"
 * detect("deno is not node"); // null
 * ```
 */
export declare function detect(content: string): typeof EOL | null;
/**
 * Normalize the input string to the targeted EOL.
 *
 * @param content The input string to normalize.
 * @param eol The EOL character(s) to normalize the input string to.
 *
 * @returns The input string normalized to the targeted EOL.
 *
 * @example Usage
 * ```ts no-eval
 * import { LF, format } from "@std/fs/eol";
 *
 * const CRLFinput = "deno\r\nis not\r\nnode";
 *
 * format(CRLFinput, LF); // "deno\nis not\nnode"
 * ```
 */
export declare function format(content: string, eol: typeof EOL): string;
//# sourceMappingURL=eol.d.ts.map