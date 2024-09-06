var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var _Spinner_spinner, _Spinner_interval, _Spinner_color, _Spinner_intervalId, _Spinner_active;
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import * as dntShim from "../../../../../../_dnt.shims.js";
import { getNoColor } from "../../../cliffy-internal/1.0.0-rc.5/runtime/no_color.js";
import { writeSync } from "../../../cliffy-internal/1.0.0-rc.5/runtime/write_sync.js";
const encoder = new TextEncoder();
const LINE_CLEAR = encoder.encode("\r\u001b[K"); // From cli/prompt_secret.ts
const COLOR_RESET = "\u001b[0m";
const DEFAULT_INTERVAL = 75;
const DEFAULT_SPINNER = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const COLORS = {
    black: "\u001b[30m",
    red: "\u001b[31m",
    green: "\u001b[32m",
    yellow: "\u001b[33m",
    blue: "\u001b[34m",
    magenta: "\u001b[35m",
    cyan: "\u001b[36m",
    white: "\u001b[37m",
    gray: "\u001b[90m",
};
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
export class Spinner {
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
    constructor({ spinner = DEFAULT_SPINNER, message = "", interval = DEFAULT_INTERVAL, color, } = {}) {
        _Spinner_spinner.set(this, void 0);
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
        Object.defineProperty(this, "message", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        _Spinner_interval.set(this, void 0);
        _Spinner_color.set(this, void 0);
        _Spinner_intervalId.set(this, void 0);
        _Spinner_active.set(this, false);
        __classPrivateFieldSet(this, _Spinner_spinner, spinner, "f");
        this.message = message;
        __classPrivateFieldSet(this, _Spinner_interval, interval, "f");
        this.color = color;
    }
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
    set color(value) {
        __classPrivateFieldSet(this, _Spinner_color, value ? COLORS[value] : undefined, "f");
    }
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
    get color() {
        return __classPrivateFieldGet(this, _Spinner_color, "f");
    }
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
    start() {
        // deno-lint-ignore no-explicit-any
        if (__classPrivateFieldGet(this, _Spinner_active, "f") || dntShim.dntGlobalThis.Deno?.stdout.writable.locked) {
            return;
        }
        __classPrivateFieldSet(this, _Spinner_active, true, "f");
        let i = 0;
        const noColor = getNoColor();
        // Updates the spinner after the given interval.
        const updateFrame = () => {
            const color = __classPrivateFieldGet(this, _Spinner_color, "f") ?? "";
            const frame = encoder.encode(noColor
                ? __classPrivateFieldGet(this, _Spinner_spinner, "f")[i] + " " + this.message
                : color + __classPrivateFieldGet(this, _Spinner_spinner, "f")[i] + COLOR_RESET + " " + this.message);
            // call writeSync once to reduce flickering
            const writeData = new Uint8Array(LINE_CLEAR.length + frame.length);
            writeData.set(LINE_CLEAR);
            writeData.set(frame, LINE_CLEAR.length);
            writeSync(writeData);
            i = (i + 1) % __classPrivateFieldGet(this, _Spinner_spinner, "f").length;
        };
        __classPrivateFieldSet(this, _Spinner_intervalId, setInterval(updateFrame, __classPrivateFieldGet(this, _Spinner_interval, "f")), "f");
    }
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
    stop() {
        if (__classPrivateFieldGet(this, _Spinner_intervalId, "f") && __classPrivateFieldGet(this, _Spinner_active, "f")) {
            clearInterval(__classPrivateFieldGet(this, _Spinner_intervalId, "f"));
            writeSync(LINE_CLEAR); // Clear the current line
            __classPrivateFieldSet(this, _Spinner_active, false, "f");
        }
    }
}
_Spinner_spinner = new WeakMap(), _Spinner_interval = new WeakMap(), _Spinner_color = new WeakMap(), _Spinner_intervalId = new WeakMap(), _Spinner_active = new WeakMap();
