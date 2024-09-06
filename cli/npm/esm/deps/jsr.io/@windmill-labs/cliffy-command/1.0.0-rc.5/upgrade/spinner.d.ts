/**
 * This is a hack to allow us to use the same type for both the color name and
 * an ANSI escape code.
 *
 * @see {@link https://github.com/microsoft/TypeScript/issues/29729#issuecomment-460346421}
 *
 * @internal
 */
export type Ansi = string & {};
/** Color options for {@linkcode SpinnerOptions.color}. */
export type Color = "black" | "red" | "green" | "yellow" | "blue" | "magenta" | "cyan" | "white" | "gray" | Ansi;
/** Options for {@linkcode Spinner}. */
export interface SpinnerOptions {
    /**
     * The sequence of characters to be iterated through for animation.
     *
     * @default {["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]}
     */
    spinner?: string[];
    /**
     * The message to display next to the spinner. This can be changed while the
     * spinner is active.
     */
    message?: string;
    /**
     * The time between each frame of the spinner in milliseconds.
     *
     * @default {75}
     */
    interval?: number;
    /**
     * The color of the spinner. Defaults to the default terminal color.
     * This can be changed while the spinner is active.
     */
    color?: Color;
}
/**
 * A spinner that can be used to indicate that something is loading.
 *
 * @example Usage
 * ```ts no-eval
 * import { Spinner } from "./spinner.ts";
 *
 * const spinner = new Spinner({ message: "Loading...", color: "yellow" });
 * spinner.start();
 *
 * setTimeout(() => {
 *  spinner.stop();
 *  console.log("Finished loading!");
 * }, 3_000);
 * ```
 */
export declare class Spinner {
    #private;
    /**
     * The message to display next to the spinner.
     * This can be changed while the spinner is active.
     *
     * @example Usage
     * ```ts no-eval
     * import { Spinner } from "./spinner.ts";
     *
     * const spinner = new Spinner({ message: "Working..." });
     * spinner.start();
     *
     * for (let step = 0; step < 5; step++) {
     *   // do some work
     *   await new Promise((resolve) => setTimeout(resolve, 1000));
     *
     *   spinner.message = `Finished Step #${step}`;
     * }
     *
     * spinner.stop();
     * console.log("Done!");
     * ```
     */
    message: string;
    /**
     * Creates a new spinner.
     *
     * @example Usage
     * ```ts no-assert
     * import { Spinner } from "./spinner.ts";
     *
     * const spinner = new Spinner({ message: "Loading..." });
     * spinner.stop();
     * ```
     */
    constructor({ spinner, message, interval, color, }?: SpinnerOptions);
    /**
     * Set the color of the spinner. This defaults to the default terminal color.
     * This can be changed while the spinner is active.
     *
     * Providing `undefined` will use the default terminal color.
     *
     * @param value Color to set.
     *
     * @example Usage
     * ```ts no-eval
     * import { Spinner } from "./spinner.ts";
     *
     * const spinner = new Spinner({ message: "Loading...", color: "yellow" });
     * spinner.start();
     *
     * // do some work
     * await new Promise((resolve) => setTimeout(resolve, 1000));
     *
     * spinner.color = "magenta";
     * ```
     */
    set color(value: Color | undefined);
    /**
     * Get the current color of the spinner.
     *
     * @example Usage
     * ```ts no-assert
     * import { Spinner } from "./spinner.ts";
     *
     * const spinner = new Spinner({ message: "Loading", color: "blue" });
     *
     * spinner.color; // Blue ANSI escape sequence
     * ```
     * @returns The color of the spinner or `undefined` if it's using the terminal default.
     */
    get color(): Color | undefined;
    /**
     * Starts the spinner.
     *
     * @example Usage
     * ```ts no-eval
     * import { Spinner } from "./spinner.ts";
     *
     * const spinner = new Spinner({ message: "Loading..." });
     * spinner.start();
     * ```
     */
    start(): void;
    /**
     * Stops the spinner.
     *
     * @example Usage
     * ```ts no-eval
     * import { Spinner } from "./spinner.ts";
     *
     * const spinner = new Spinner({ message: "Loading..." });
     * spinner.start();
     *
     * setTimeout(() => {
     *  spinner.stop();
     *  console.log("Finished loading!");
     * }, 3_000);
     * ```
     */
    stop(): void;
}
//# sourceMappingURL=spinner.d.ts.map