import { GenericPrompt } from "./_generic_prompt.js";
import { GenericSuggestions, } from "./_generic_suggestions.js";
import { dim } from "../../../@std/fmt/0.225.6/colors.js";
/**
 * Confirm prompt representation.
 *
 * ```ts
 * import { Confirm } from "./mod.ts";
 *
 * const confirmed: boolean = await Confirm.prompt("Please confirm");
 * ```
 */
export class Confirm extends GenericSuggestions {
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
            active: options.active || "Yes",
            inactive: options.inactive || "No",
            files: false,
            complete: undefined,
            suggestions: [
                options.active || "Yes",
                options.inactive || "No",
            ],
            list: false,
            info: false,
        };
    }
    defaults() {
        let defaultMessage = "";
        if (this.settings.default === true) {
            defaultMessage += this.settings.active[0].toUpperCase() + "/" +
                this.settings.inactive[0].toLowerCase();
        }
        else if (this.settings.default === false) {
            defaultMessage += this.settings.active[0].toLowerCase() + "/" +
                this.settings.inactive[0].toUpperCase();
        }
        else {
            defaultMessage += this.settings.active[0].toLowerCase() + "/" +
                this.settings.inactive[0].toLowerCase();
        }
        return defaultMessage ? dim(` (${defaultMessage})`) : "";
    }
    success(value) {
        this.saveSuggestions(this.format(value));
        return super.success(value);
    }
    /** Get input input. */
    getValue() {
        return this.inputValue;
    }
    /**
     * Validate input value.
     * @param value User input value.
     * @return True on success, false or error message on error.
     */
    validate(value) {
        return typeof value === "string" &&
            [
                this.settings.active[0].toLowerCase(),
                this.settings.active.toLowerCase(),
                this.settings.inactive[0].toLowerCase(),
                this.settings.inactive.toLowerCase(),
            ].indexOf(value.toLowerCase()) !== -1;
    }
    /**
     * Map input value to output value.
     * @param value Input value.
     * @return Output value.
     */
    transform(value) {
        switch (value.toLowerCase()) {
            case this.settings.active[0].toLowerCase():
            case this.settings.active.toLowerCase():
                return true;
            case this.settings.inactive[0].toLowerCase():
            case this.settings.inactive.toLowerCase():
                return false;
        }
        return;
    }
    /**
     * Format output value.
     * @param value Output value.
     */
    format(value) {
        return value ? this.settings.active : this.settings.inactive;
    }
}
