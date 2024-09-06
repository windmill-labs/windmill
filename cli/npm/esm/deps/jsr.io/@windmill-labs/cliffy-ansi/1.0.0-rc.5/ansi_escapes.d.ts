/** Ring audio bell: `\u0007` */
export declare const bel = "\u0007";
/** Get cursor position. */
export declare const cursorPosition: string;
/**
 * Move cursor to x, y, counting from the top left corner.
 * @param x Position left.
 * @param y Position top.
 */
export declare function cursorTo(x: number, y?: number): string;
/**
 * Move cursor by offset.
 * @param x Offset left.
 * @param y Offset top.
 */
export declare function cursorMove(x: number, y: number): string;
/**
 * Move cursor up by n lines.
 * @param count Number of lines.
 */
export declare function cursorUp(count?: number): string;
/**
 * Move cursor down by n lines.
 * @param count Number of lines.
 */
export declare function cursorDown(count?: number): string;
/**
 * Move cursor forward by n lines.
 * @param count Number of lines.
 */
export declare function cursorForward(count?: number): string;
/**
 * Move cursor backward by n lines.
 * @param count Number of lines.
 */
export declare function cursorBackward(count?: number): string;
/**
 * Move cursor to the beginning of the line n lines down.
 * @param count Number of lines.
 */
export declare function cursorNextLine(count?: number): string;
/**
 * Move cursor to the beginning of the line n lines up.
 * @param count Number of lines.
 */
export declare function cursorPrevLine(count?: number): string;
/** Move cursor to first column of current row. */
export declare const cursorLeft: string;
/** Hide cursor. */
export declare const cursorHide: string;
/** Show cursor. */
export declare const cursorShow: string;
/** Save cursor. */
export declare const cursorSave: string;
/** Restore cursor. */
export declare const cursorRestore: string;
/**
 * Scroll window up by n lines.
 * @param count Number of lines.
 */
export declare function scrollUp(count?: number): string;
/**
 * Scroll window down by n lines.
 * @param count Number of lines.
 */
export declare function scrollDown(count?: number): string;
/** Clear screen. */
export declare const eraseScreen: string;
/**
 * Clear screen up by n lines.
 * @param count Number of lines.
 */
export declare function eraseUp(count?: number): string;
/**
 * Clear screen down by n lines.
 * @param count Number of lines.
 */
export declare function eraseDown(count?: number): string;
/** Clear current line. */
export declare const eraseLine: string;
/** Clear to line end. */
export declare const eraseLineEnd: string;
/** Clear to line start. */
export declare const eraseLineStart: string;
/**
 * Clear screen and move cursor by n lines up and move cursor to first column.
 * @param count Number of lines.
 */
export declare function eraseLines(count: number): string;
/** Clear the terminal screen. (Viewport) */
export declare const clearScreen = "\u001Bc";
/**
 * Clear the whole terminal, including scrollback buffer.
 * (Not just the visible part of it).
 */
export declare const clearTerminal: string;
/**
 * Create link.
 *
 * @param text Link text.
 * @param url Link url.
 *
 * ```ts
 * import { link } from "@cliffy/ansi/ansi-escapes";
 *
 * console.log(
 *   link("Click me.", "https://deno.land"),
 * );
 * ```
 */
export declare function link(text: string, url: string): string;
/** Image options. */
export interface ImageOptions {
    width?: number;
    height?: number;
    preserveAspectRatio?: boolean;
}
/**
 * Create image.
 *
 * @param buffer  Image buffer.
 * @param options Image options.
 *
 * ```ts
 * import { image } from "@cliffy/ansi/ansi-escapes";
 *
 * const response = await fetch("https://deno.land/images/hashrock_simple.png");
 * const imageBuffer: ArrayBuffer = await response.arrayBuffer();
 * console.log(
 *   image(imageBuffer),
 * );
 * ```
 */
export declare function image(buffer: string | ArrayBuffer, options?: ImageOptions): string;
//# sourceMappingURL=ansi_escapes.d.ts.map