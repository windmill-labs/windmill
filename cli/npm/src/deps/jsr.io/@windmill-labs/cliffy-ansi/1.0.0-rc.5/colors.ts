import * as stdColors from "../../../@std/fmt/0.225.6/colors.js";

type ExcludedColorMethods = "setColorEnabled" | "getColorEnabled";
type PropertyNames = keyof typeof stdColors;
type ColorMethod = (str: string, ...args: Array<unknown>) => string;
type ColorMethods = Exclude<PropertyNames, ExcludedColorMethods>;
type Chainable<T, E extends keyof T | null = null> = {
  [P in keyof T]: P extends E ? T[P] : Chainable<T, E> & T[P];
};

/** Chainable colors instance returned by all ansi escape properties. */
export type ColorsChain =
  & Chainable<typeof stdColors, ExcludedColorMethods>
  & { _stack: Array<ColorMethods> };

/** Create new `Colors` instance. */
export type ColorsFactory = () => Colors;

/**
 * Chainable colors module. If invoked as method, a new `Colors` instance will
 * be returned.
 */
export type Colors = ColorsFactory & ColorsChain;

const proto = Object.create(null);
const methodNames = Object.keys(stdColors) as Array<PropertyNames>;
for (const name of methodNames) {
  if (name === "setColorEnabled" || name === "getColorEnabled") {
    continue;
  }
  Object.defineProperty(proto, name, {
    get(this: ColorsChain) {
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
export const colors: Colors = factory();

function factory(stack: Array<ColorMethods> = []): Colors {
  const colors: Colors = function (
    this: ColorsChain | undefined,
    str?: string,
    ...args: Array<unknown>
  ): string | ColorsChain {
    if (typeof str !== "undefined") {
      const lastIndex = stack.length - 1;
      return stack.reduce(
        (str: string, name: PropertyNames, index: number) =>
          index === lastIndex
            ? (stdColors[name] as ColorMethod)(str, ...args)
            : (stdColors[name] as ColorMethod)(str),
        str,
      );
    }
    const tmp = stack.slice();
    stack = [];
    return factory(tmp);
  } as Colors;

  Object.setPrototypeOf(colors, proto);
  colors._stack = stack;
  return colors;
}
