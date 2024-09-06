import { GenericPrompt } from "./_generic_prompt.js";
import { underline } from "../../../@std/fmt/0.225.6/colors.js";
import { GenericInput, } from "./_generic_input.js";
/**
 * Secret prompt representation.
 *
 * ```ts
 * import { Secret } from "./mod.ts";
 *
 * const password: string = await Secret.prompt("Enter your password");
 * ```
 */
export class Secret extends GenericInput {
    /** Execute the prompt with provided message or options. */
    static prompt(options) {
        return new this(options).prompt();
    }
    /**
     * Inject prompt value. If called, the prompt doesn't prompt for an input and
     * returns immediately the injected value. Can be used for unit tests or pre
     * selections.
     *
     * @param value Input value.
     */
    static inject(value) {
        GenericPrompt.inject(value);
    }
    constructor(options) {
        super();
        Object.defineProperty(this, "settings", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        if (typeof options === "string") {
            options = { message: options };
        }
        this.settings = this.getDefaultSettings(options);
    }
    getDefaultSettings(options) {
        return {
            ...super.getDefaultSettings(options),
            label: options.label ?? "Secret",
            hidden: options.hidden ?? false,
            minLength: options.minLength ?? 0,
            maxLength: options.maxLength ?? Infinity,
        };
    }
    input() {
        return underline(this.settings.hidden ? "" : "*".repeat(this.inputValue.length));
    }
    /** Read user input. */
    read() {
        if (this.settings.hidden) {
            this.settings.tty.cursorHide();
        }
        return super.read();
    }
    /**
     * Validate input value.
     * @param value User input value.
     * @return True on success, false or error message on error.
     */
    validate(value) {
        if (typeof value !== "string") {
            return false;
        }
        if (value.length < this.settings.minLength) {
            return `${this.settings.label} must be longer than ${this.settings.minLength} but has a length of ${value.length}.`;
        }
        if (value.length > this.settings.maxLength) {
            return `${this.settings.label} can't be longer than ${this.settings.maxLength} but has a length of ${value.length}.`;
        }
        return true;
    }
    /**
     * Map input value to output value.
     * @param value Input value.
     * @return Output value.
     */
    transform(value) {
        return value;
    }
    /**
     * Format output value.
     * @param value Output value.
     */
    format(value) {
        return this.settings.hidden ? "*".repeat(8) : "*".repeat(value.length);
    }
    /** Get input input. */
    getValue() {
        return this.inputValue;
    }
}
