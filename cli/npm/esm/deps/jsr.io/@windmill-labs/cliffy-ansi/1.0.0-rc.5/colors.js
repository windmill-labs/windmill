import * as stdColors from "../../../@std/fmt/0.225.6/colors.js";
const proto = Object.create(null);
const methodNames = Object.keys(stdColors);
for (const name of methodNames) {
    if (name === "setColorEnabled" || name === "getColorEnabled") {
        continue;
    }
    Object.defineProperty(proto, name, {
        get() {
            return factory([...this._stack, name]);
        },
    });
}
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
export const colors = factory();
function factory(stack = []) {
    const colors = function (str, ...args) {
        if (typeof str !== "undefined") {
            const lastIndex = stack.length - 1;
            return stack.reduce((str, name, index) => index === lastIndex
                ? stdColors[name](str, ...args)
                : stdColors[name](str), str);
        }
        const tmp = stack.slice();
        stack = [];
        return factory(tmp);
    };
    Object.setPrototypeOf(colors, proto);
    colors._stack = stack;
    return colors;
}
