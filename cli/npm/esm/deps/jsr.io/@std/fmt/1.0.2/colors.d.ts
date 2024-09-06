/** RGB 8-bits per channel. Each in range `0->255` or `0x00->0xff` */
export interface Rgb {
    /** Red component value */
    r: number;
    /** Green component value */
    g: number;
    /** Blue component value */
    b: number;
}
/**
 * Enable or disable text color when styling.
 *
 * `@std/fmt/colors` automatically detects NO_COLOR environmental variable
 * and disables text color. Use this API only when the automatic detection
 * doesn't work.
 *
 * @example Usage
 * ```ts no-assert
 * import { setColorEnabled } from "@std/fmt/colors";
 *
 * // Disable text color
 * setColorEnabled(false);
 *
 * // Enable text color
 * setColorEnabled(true);
 * ```
 *
 * @param value The boolean value to enable or disable text color
 */
export declare function setColorEnabled(value: boolean): void;
/**
 * Get whether text color change is enabled or disabled.
 *
 * @example Usage
 * ```ts no-assert
 * import { getColorEnabled } from "@std/fmt/colors";
 *
 * console.log(getColorEnabled()); // true if enabled, false if disabled
 * ```
 * @returns `true` if text color is enabled, `false` otherwise
 */
export declare function getColorEnabled(): boolean;
/**
 * Reset the text modified.
 *
 * @example Usage
 * ```ts no-assert
 * import { reset } from "@std/fmt/colors";
 *
 * console.log(reset("Hello, world!"));
 * ```
 *
 * @param str The text to reset
 * @returns The text with reset color
 */
export declare function reset(str: string): string;
/**
 * Make the text bold.
 *
 * @example Usage
 * ```ts no-assert
 * import { bold } from "@std/fmt/colors";
 *
 * console.log(bold("Hello, world!"));
 * ```
 *
 * @param str The text to make bold
 * @returns The bold text
 */
export declare function bold(str: string): string;
/**
 * The text emits only a small amount of light.
 *
 * @example Usage
 * ```ts no-assert
 * import { dim } from "@std/fmt/colors";
 *
 * console.log(dim("Hello, world!"));
 * ```
 *
 * @param str The text to dim
 * @returns The dimmed text
 *
 * Warning: Not all terminal emulators support `dim`.
 * For compatibility across all terminals, use {@linkcode gray} or {@linkcode brightBlack} instead.
 */
export declare function dim(str: string): string;
/**
 * Make the text italic.
 *
 * @example Usage
 * ```ts no-assert
 * import { italic } from "@std/fmt/colors";
 *
 * console.log(italic("Hello, world!"));
 * ```
 *
 * @param str The text to make italic
 * @returns The italic text
 */
export declare function italic(str: string): string;
/**
 * Make the text underline.
 *
 * @example Usage
 * ```ts no-assert
 * import { underline } from "@std/fmt/colors";
 *
 * console.log(underline("Hello, world!"));
 * ```
 *
 * @param str The text to underline
 * @returns The underlined text
 */
export declare function underline(str: string): string;
/**
 * Invert background color and text color.
 *
 * @example Usage
 * ```ts no-assert
 * import { inverse } from "@std/fmt/colors";
 *
 * console.log(inverse("Hello, world!"));
 * ```
 *
 * @param str The text to invert its color
 * @returns The inverted text
 */
export declare function inverse(str: string): string;
/**
 * Make the text hidden.
 *
 * @example Usage
 * ```ts no-assert
 * import { hidden } from "@std/fmt/colors";
 *
 * console.log(hidden("Hello, world!"));
 * ```
 *
 * @param str The text to hide
 * @returns The hidden text
 */
export declare function hidden(str: string): string;
/**
 * Put horizontal line through the center of the text.
 *
 * @example Usage
 * ```ts no-assert
 * import { strikethrough } from "@std/fmt/colors";
 *
 * console.log(strikethrough("Hello, world!"));
 * ```
 *
 * @param str The text to strike through
 * @returns The text with horizontal line through the center
 */
export declare function strikethrough(str: string): string;
/**
 * Set text color to black.
 *
 * @example Usage
 * ```ts no-assert
 * import { black } from "@std/fmt/colors";
 *
 * console.log(black("Hello, world!"));
 * ```
 *
 * @param str The text to make black
 * @returns The black text
 */
export declare function black(str: string): string;
/**
 * Set text color to red.
 *
 * @example Usage
 * ```ts no-assert
 * import { red } from "@std/fmt/colors";
 *
 * console.log(red("Hello, world!"));
 * ```
 *
 * @param str The text to make red
 * @returns The red text
 */
export declare function red(str: string): string;
/**
 * Set text color to green.
 *
 * @example Usage
 * ```ts no-assert
 * import { green } from "@std/fmt/colors";
 *
 * console.log(green("Hello, world!"));
 * ```
 *
 * @param str The text to make green
 * @returns The green text
 */
export declare function green(str: string): string;
/**
 * Set text color to yellow.
 *
 * @example Usage
 * ```ts no-assert
 * import { yellow } from "@std/fmt/colors";
 *
 * console.log(yellow("Hello, world!"));
 * ```
 *
 * @param str The text to make yellow
 * @returns The yellow text
 */
export declare function yellow(str: string): string;
/**
 * Set text color to blue.
 *
 * @example Usage
 * ```ts no-assert
 * import { blue } from "@std/fmt/colors";
 *
 * console.log(blue("Hello, world!"));
 * ```
 *
 * @param str The text to make blue
 * @returns The blue text
 */
export declare function blue(str: string): string;
/**
 * Set text color to magenta.
 *
 * @example Usage
 * ```ts no-assert
 * import { magenta } from "@std/fmt/colors";
 *
 * console.log(magenta("Hello, world!"));
 * ```
 *
 * @param str The text to make magenta
 * @returns The magenta text
 */
export declare function magenta(str: string): string;
/**
 * Set text color to cyan.
 *
 * @example Usage
 * ```ts no-assert
 * import { cyan } from "@std/fmt/colors";
 *
 * console.log(cyan("Hello, world!"));
 * ```
 *
 * @param str The text to make cyan
 * @returns The cyan text
 */
export declare function cyan(str: string): string;
/**
 * Set text color to white.
 *
 * @example Usage
 * ```ts no-assert
 * import { white } from "@std/fmt/colors";
 *
 * console.log(white("Hello, world!"));
 * ```
 *
 * @param str The text to make white
 * @returns The white text
 */
export declare function white(str: string): string;
/**
 * Set text color to gray.
 *
 * @example Usage
 * ```ts no-assert
 * import { gray } from "@std/fmt/colors";
 *
 * console.log(gray("Hello, world!"));
 * ```
 *
 * @param str The text to make gray
 * @returns The gray text
 */
export declare function gray(str: string): string;
/**
 * Set text color to bright black.
 *
 * @example Usage
 * ```ts no-assert
 * import { brightBlack } from "@std/fmt/colors";
 *
 * console.log(brightBlack("Hello, world!"));
 * ```
 *
 * @param str The text to make bright black
 * @returns The bright black text
 */
export declare function brightBlack(str: string): string;
/**
 * Set text color to bright red.
 *
 * @example Usage
 * ```ts no-assert
 * import { brightRed } from "@std/fmt/colors";
 *
 * console.log(brightRed("Hello, world!"));
 * ```
 *
 * @param str The text to make bright red
 * @returns The bright red text
 */
export declare function brightRed(str: string): string;
/**
 * Set text color to bright green.
 *
 * @example Usage
 * ```ts no-assert
 * import { brightGreen } from "@std/fmt/colors";
 *
 * console.log(brightGreen("Hello, world!"));
 * ```
 *
 * @param str The text to make bright green
 * @returns The bright green text
 */
export declare function brightGreen(str: string): string;
/**
 * Set text color to bright yellow.
 *
 * @example Usage
 * ```ts no-assert
 * import { brightYellow } from "@std/fmt/colors";
 *
 * console.log(brightYellow("Hello, world!"));
 * ```
 *
 * @param str The text to make bright yellow
 * @returns The bright yellow text
 */
export declare function brightYellow(str: string): string;
/**
 * Set text color to bright blue.
 *
 * @example Usage
 * ```ts no-assert
 * import { brightBlue } from "@std/fmt/colors";
 *
 * console.log(brightBlue("Hello, world!"));
 * ```
 *
 * @param str The text to make bright blue
 * @returns The bright blue text
 */
export declare function brightBlue(str: string): string;
/**
 * Set text color to bright magenta.
 *
 * @example Usage
 * ```ts no-assert
 * import { brightMagenta } from "@std/fmt/colors";
 *
 * console.log(brightMagenta("Hello, world!"));
 * ```
 *
 * @param str The text to make bright magenta
 * @returns The bright magenta text
 */
export declare function brightMagenta(str: string): string;
/**
 * Set text color to bright cyan.
 *
 * @example Usage
 * ```ts no-assert
 * import { brightCyan } from "@std/fmt/colors";
 *
 * console.log(brightCyan("Hello, world!"));
 * ```
 *
 * @param str The text to make bright cyan
 * @returns The bright cyan text
 */
export declare function brightCyan(str: string): string;
/**
 * Set text color to bright white.
 *
 * @example Usage
 * ```ts no-assert
 * import { brightWhite } from "@std/fmt/colors";
 *
 * console.log(brightWhite("Hello, world!"));
 * ```
 *
 * @param str The text to make bright white
 * @returns The bright white text
 */
export declare function brightWhite(str: string): string;
/**
 * Set background color to black.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBlack } from "@std/fmt/colors";
 *
 * console.log(bgBlack("Hello, world!"));
 * ```
 *
 * @param str The text to make its background black
 * @returns The text with black background
 */
export declare function bgBlack(str: string): string;
/**
 * Set background color to red.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgRed } from "@std/fmt/colors";
 *
 * console.log(bgRed("Hello, world!"));
 * ```
 *
 * @param str The text to make its background red
 * @returns The text with red background
 */
export declare function bgRed(str: string): string;
/**
 * Set background color to green.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgGreen } from "@std/fmt/colors";
 *
 * console.log(bgGreen("Hello, world!"));
 * ```
 *
 * @param str The text to make its background green
 * @returns The text with green background
 */
export declare function bgGreen(str: string): string;
/**
 * Set background color to yellow.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgYellow } from "@std/fmt/colors";
 *
 * console.log(bgYellow("Hello, world!"));
 * ```
 *
 * @param str The text to make its background yellow
 * @returns The text with yellow background
 */
export declare function bgYellow(str: string): string;
/**
 * Set background color to blue.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBlue } from "@std/fmt/colors";
 *
 * console.log(bgBlue("Hello, world!"));
 * ```
 *
 * @param str The text to make its background blue
 * @returns The text with blue background
 */
export declare function bgBlue(str: string): string;
/**
 *  Set background color to magenta.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgMagenta } from "@std/fmt/colors";
 *
 * console.log(bgMagenta("Hello, world!"));
 * ```
 *
 * @param str The text to make its background magenta
 * @returns The text with magenta background
 */
export declare function bgMagenta(str: string): string;
/**
 * Set background color to cyan.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgCyan } from "@std/fmt/colors";
 *
 * console.log(bgCyan("Hello, world!"));
 * ```
 *
 * @param str The text to make its background cyan
 * @returns The text with cyan background
 */
export declare function bgCyan(str: string): string;
/**
 * Set background color to white.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgWhite } from "@std/fmt/colors";
 *
 * console.log(bgWhite("Hello, world!"));
 * ```
 *
 * @param str The text to make its background white
 * @returns The text with white background
 */
export declare function bgWhite(str: string): string;
/**
 * Set background color to bright black.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBrightBlack } from "@std/fmt/colors";
 *
 * console.log(bgBrightBlack("Hello, world!"));
 * ```
 *
 * @param str The text to make its background bright black
 * @returns The text with bright black background
 */
export declare function bgBrightBlack(str: string): string;
/**
 * Set background color to bright red.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBrightRed } from "@std/fmt/colors";
 *
 * console.log(bgBrightRed("Hello, world!"));
 * ```
 *
 * @param str The text to make its background bright red
 * @returns The text with bright red background
 */
export declare function bgBrightRed(str: string): string;
/**
 * Set background color to bright green.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBrightGreen } from "@std/fmt/colors";
 *
 * console.log(bgBrightGreen("Hello, world!"));
 * ```
 *
 * @param str The text to make its background bright green
 * @returns The text with bright green background
 */
export declare function bgBrightGreen(str: string): string;
/**
 * Set background color to bright yellow.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBrightYellow } from "@std/fmt/colors";
 *
 * console.log(bgBrightYellow("Hello, world!"));
 * ```
 *
 * @param str The text to make its background bright yellow
 * @returns The text with bright yellow background
 */
export declare function bgBrightYellow(str: string): string;
/**
 * Set background color to bright blue.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBrightBlue } from "@std/fmt/colors";
 *
 * console.log(bgBrightBlue("Hello, world!"));
 * ```
 *
 * @param str The text to make its background bright blue
 * @returns The text with bright blue background
 */
export declare function bgBrightBlue(str: string): string;
/**
 * Set background color to bright magenta.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBrightMagenta } from "@std/fmt/colors";
 *
 * console.log(bgBrightMagenta("Hello, world!"));
 * ```
 *
 * @param str The text to make its background bright magenta
 * @returns The text with bright magenta background
 */
export declare function bgBrightMagenta(str: string): string;
/**
 * Set background color to bright cyan.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBrightCyan } from "@std/fmt/colors";
 *
 * console.log(bgBrightCyan("Hello, world!"));
 * ```
 *
 * @param str The text to make its background bright cyan
 * @returns The text with bright cyan background
 */
export declare function bgBrightCyan(str: string): string;
/**
 * Set background color to bright white.
 *
 * @example Usage
 * ```ts no-assert
 * import { bgBrightWhite } from "@std/fmt/colors";
 *
 * console.log(bgBrightWhite("Hello, world!"));
 * ```
 *
 * @param str The text to make its background bright white
 * @returns The text with bright white background
 */
export declare function bgBrightWhite(str: string): string;
/**
 * Set text color using paletted 8bit colors.
 * https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
 *
 * @example Usage
 * ```ts no-assert
 * import { rgb8 } from "@std/fmt/colors";
 *
 * console.log(rgb8("Hello, world!", 42));
 * ```
 *
 * @param str The text color to apply paletted 8bit colors to
 * @param color The color code
 * @returns The text with paletted 8bit color
 */
export declare function rgb8(str: string, color: number): string;
/**
 * Set background color using paletted 8bit colors.
 * https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
 *
 * @example Usage
 * ```ts no-assert
 * import { bgRgb8 } from "@std/fmt/colors";
 *
 * console.log(bgRgb8("Hello, world!", 42));
 * ```
 *
 * @param str The text color to apply paletted 8bit background colors to
 * @param color code
 * @returns The text with paletted 8bit background color
 */
export declare function bgRgb8(str: string, color: number): string;
/**
 * Set text color using 24bit rgb.
 * `color` can be a number in range `0x000000` to `0xffffff` or
 * an `Rgb`.
 *
 * @example To produce the color magenta:
 * ```ts no-assert
 * import { rgb24 } from "@std/fmt/colors";
 *
 * rgb24("foo", 0xff00ff);
 * rgb24("foo", {r: 255, g: 0, b: 255});
 * ```
 * @param str The text color to apply 24bit rgb to
 * @param color The color code
 * @returns The text with 24bit rgb color
 */
export declare function rgb24(str: string, color: number | Rgb): string;
/**
 * Set background color using 24bit rgb.
 * `color` can be a number in range `0x000000` to `0xffffff` or
 * an `Rgb`.
 *
 * @example To produce the color magenta:
 * ```ts no-assert
 * import { bgRgb24 } from "@std/fmt/colors";
 *
 * bgRgb24("foo", 0xff00ff);
 * bgRgb24("foo", {r: 255, g: 0, b: 255});
 * ```
 * @param str The text color to apply 24bit rgb to
 * @param color The color code
 * @returns The text with 24bit rgb color
 */
export declare function bgRgb24(str: string, color: number | Rgb): string;
/**
 * Remove ANSI escape codes from the string.
 *
 * @example Usage
 * ```ts no-assert
 * import { stripAnsiCode, red } from "@std/fmt/colors";
 *
 * console.log(stripAnsiCode(red("Hello, world!")));
 * ```
 *
 * @param string The text to remove ANSI escape codes from
 * @returns The text without ANSI escape codes
 */
export declare function stripAnsiCode(string: string): string;
//# sourceMappingURL=colors.d.ts.map