/**
 * Marks a function as deprecated and emit warnings when it is called.
 * @module
 * @experimental
 */

import runtime, { RuntimeInfo } from "./runtime.ts";
import wrap from "./wrap.ts";

const warnedRecord = new Map<Function, boolean>();

/**
 * Marks a function as deprecated and returns a wrapped function.
 * 
 * When the wrapped function is called, a deprecation warning will be emitted
 * to the stdout.
 * 
 * **NOTE:** The original function must have a name.
 * 
 * **NOTE:** This function is **experimental** and the warning may point to the
 * wrong file source in some environments.
 * 
 * @param tip Extra tip for the user to migrate.
 * @param once If set, the warning will only be emitted once.
 * 
 * @example
 * ```ts
 * import deprecate from "@ayonli/jsext/deprecate";
 * 
 * const sum = deprecate(function sum(a: number, b: number) {
 *     return a + b;
 * }, "use `a + b` instead");
 * console.log(sum(1, 2));
 * // output:
 * // DeprecationWarning: sum() is deprecated, use `a + b` instead (at <anonymous>:4:13)
 * // 3
 * ```
 */
export default function deprecate<T, Fn extends (this: T, ...args: any[]) => any>(
    fn: Fn,
    tip?: string,
    once?: boolean
): Fn;
/**
 * Emits a deprecation warning for the target, usually a parameter, an option,
 * or the function's name, etc.
 * 
 * @param forFn Usually set to the current function, used to locate the call-site.
 * @param tip Extra tip for the user to migrate.
 * @param once If set, the warning will only be emitted once.
 * 
 * @example
 * ```ts
 * import deprecate from "@ayonli/jsext/deprecate";
 * 
 * function pow(a: number, b: number) {
 *     deprecate("pow()", pow, "use `a ** b` instead");
 *     return a ** b;
 * }
 * 
 * console.log(pow(2, 3));
 * // output:
 * // DeprecationWarning: pow() is deprecated, use `a ** b` instead (at <anonymous>:5:13)
 * // 3
 * ```
 */
export default function deprecate(target: string, forFn: Function, tip?: string, once?: boolean): void;
export default function deprecate<T, Fn extends (this: T, ...args: any[]) => any>(
    target: Fn | string,
    ...args: any[]
): Fn | void {
    const { identity } = runtime();

    if (typeof target === "function") {
        const tip = (args[0] as string) ?? "";
        const once = (args[1] as boolean) ?? false;

        return wrap<T, Fn>(target, function wrapped(fn, ...args) {
            const lineOffset = ({
                "node": 2,
                "deno": 2,
                "chrome": 2,
                "workerd": 2,
                "bun": 1,
                "safari": 1,
                "firefox": 3,
                "fastly": 3,
                "unknown": 3,
            })[identity]!;

            emitWarning(fn.name + "()", wrapped, tip, once, lineOffset, identity, true);
            return fn.apply(this, args);
        });
    }

    const forFn = args[0] as Function;
    const tip = (args[1] as string) ?? "";
    const once = (args[2] as boolean) ?? false;
    const lineOffset = ({
        "node": 1,
        "deno": 1,
        "chrome": 1,
        "workerd": 1,
        "bun": 1,
        "safari": 1,
        "firefox": 3,
        "fastly": 3,
        "unknown": 3,
    })[identity]!;

    return emitWarning(target, forFn, tip, once, lineOffset, identity, false);
}

function emitWarning(
    target: string,
    forFn: Function,
    tip: string,
    once: boolean,
    lineNum: number,
    runtime: RuntimeInfo["identity"],
    wrapped = false
) {
    if (!once || !warnedRecord.has(forFn)) {
        let trace: { stack?: string; } = {};

        if (typeof Error.captureStackTrace === "function") {
            Error.captureStackTrace(trace, forFn);
        } else {
            trace = new Error("");
        }

        let lines = (trace.stack as string).split("\n");
        const offset = lines.findIndex(line => line === "Error");

        if (offset !== -1 && offset !== 0) {
            lines = lines.slice(offset); // fix for tsx in Node.js v16
        }

        let line: string | undefined;

        if (runtime === "safari") {
            line = lines.find(line => line.trim().startsWith("module code@"))
                || lines[lineNum];
        } else if (runtime === "bun" && !wrapped) {
            line = lines.find(line => line.trim().startsWith("at module code"))
                || lines[lineNum];
        } else {
            line = lines[lineNum];
        }

        let warning = `${target} is deprecated`;

        if (tip) {
            warning += ", " + tip;
        }

        if (line) {
            line = line.trim();
            let start = line.indexOf("(");

            if (start !== -1) {
                start += 1;
                const end = line.indexOf(")", start);
                line = line.slice(start, end);
            }

            warning += " (" + line + ")";
        }

        console.warn("DeprecationWarning:", warning);
        once && warnedRecord.set(forFn, true);
    }
}
