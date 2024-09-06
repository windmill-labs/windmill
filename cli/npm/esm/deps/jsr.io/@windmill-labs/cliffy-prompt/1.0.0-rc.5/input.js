import { GenericPrompt } from "./_generic_prompt.js";
import { GenericSuggestions, } from "./_generic_suggestions.js";
import { normalize } from "../../../@std/path/1.0.0-rc.2/mod.js";
/**
 * Input prompt representation.
 *
 * ```ts
 * import { Input } from "./mod.ts";
 *
 * const confirmed: string = await Input.prompt("Enter your name");
 * ```
 */
export class Input extends GenericSuggestions {
    /** Execute the prompt with provided options. */
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
            minLength: options.minLength ?? 0,
            maxLength: options.maxLength ?? Infinity,
        };
    }
    success(value) {
        this.saveSuggestions(value);
        return super.success(value);
    }
    /** Get input value. */
    getValue() {
        return this.settings.files ? normalize(this.inputValue) : this.inputValue;
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
            return `Value must be longer than ${this.settings.minLength} but has a length of ${value.length}.`;
        }
        if (value.length > this.settings.maxLength) {
            return `Value can't be longer than ${this.settings.maxLength} but has a length of ${value.length}.`;
        }
        return true;
    }
    /**
     * Map input value to output value.
     * @param value Input value.
     * @return Output value.
     */
    transform(value) {
        return value.trim();
    }
    /**
     * Format output value.
     * @param value Output value.
     */
    format(value) {
        return value;
    }
}
