import { readSync } from "../../cliffy-internal/1.0.0-rc.5/runtime/read_sync.js";
import { setRaw } from "../../cliffy-internal/1.0.0-rc.5/runtime/set_raw.js";
import { writeSync } from "../../cliffy-internal/1.0.0-rc.5/runtime/write_sync.js";
import * as ansiEscapes from "./ansi_escapes.js";
import { getCursorPosition } from "./cursor_position.js";
/**
 * Chainable ansi escape sequences.
 * If invoked as method, a new Tty instance will be returned.
 *
 * ```ts
 * import { tty } from "@cliffy/ansi/tty";
 *
 * tty.cursorTo(0, 0).eraseScreen();
 * ```
 */
export const tty = factory();
const encoder = new TextEncoder();
function factory(options) {
    let result = "";
    let stack = [];
    const writer = options?.writer ?? { writeSync };
    const reader = options?.reader ?? { readSync, setRaw };
    const tty = function (...args) {
        if (this) {
            update(args);
            writer.writeSync(encoder.encode(result));
            return this;
        }
        return factory(args[0] ?? options);
    };
    tty.text = function (text) {
        stack.push([text, []]);
        update();
        writer.writeSync(encoder.encode(result));
        return this;
    };
    tty.getCursorPosition = () => getCursorPosition({ writer, reader });
    const methodList = Object.entries(ansiEscapes);
    for (const [name, method] of methodList) {
        if (name === "cursorPosition") {
            continue;
        }
        Object.defineProperty(tty, name, {
            get() {
                stack.push([method, []]);
                return this;
            },
        });
    }
    return tty;
    function update(args) {
        if (!stack.length) {
            return;
        }
        if (args) {
            stack[stack.length - 1][1] = args;
        }
        result = stack.reduce((prev, [cur, args]) => prev + (typeof cur === "string" ? cur : cur.call(tty, ...args)), "");
        stack = [];
    }
}
