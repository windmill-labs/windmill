import * as stdColors from "../../../@std/fmt/0.225.6/colors.js";
type ExcludedColorMethods = "setColorEnabled" | "getColorEnabled";
type PropertyNames = keyof typeof stdColors;
type ColorMethods = Exclude<PropertyNames, ExcludedColorMethods>;
type Chainable<T, E extends keyof T | null = null> = {
    [P in keyof T]: P extends E ? T[P] : Chainable<T, E> & T[P];
};
/** Chainable colors instance returned by all ansi escape properties. */
export type ColorsChain = Chainable<typeof stdColors, ExcludedColorMethods> & {
    _stack: Array<ColorMethods>;
};
/** Create new `Colors` instance. */
export type ColorsFactory = () => Colors;
/**
 * Chainable colors module. If invoked as method, a new `Colors` instance will
 * be returned.
 */
export type Colors = ColorsFactory & ColorsChain;
/**
 * Chainable colors module.
 *
 * ```ts
 * import { colors } from "@cliffy/ansi/colors";
 *
 * console.log(colors.blue.bgRed.bold('Welcome to Deno.Land!'));
 * ```
 *
 * If invoked as method, a new Ansi instance will be returned.
 *
 * ```ts
 * import { Colors, colors } from "@cliffy/ansi/colors";
 *
 * const myColors: Colors = colors();
 * console.log(myColors.blue.bgRed.bold('Welcome to Deno.Land!'));
 * ```
 */
export declare const colors: Colors;
export {};
//# sourceMappingURL=colors.d.ts.map