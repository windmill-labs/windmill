import { brightBlue, underline } from "../../../@std/fmt/0.225.6/colors.js";
import { GenericList, isOption, isOptionGroup, } from "./_generic_list.js";
import { GenericPrompt } from "./_generic_prompt.js";
import { getFiguresByKeys } from "./_figures.js";
import { equal } from "../../../@std/assert/1.0.0-rc.2/equal.js";
/**
 * Select prompt representation.
 *
 * Simple prompt:
 *
 * ```ts
 * import { Select } from "./mod.ts";
 *
 * const color: string = await Select.prompt({
 *   message: "Pick a color",
 *   options: ["red", "green", "blue"],
 * });
 * ```
 *
 * Mixed option types:
 *
 * ```ts
 * import { Select } from "./mod.ts";
 *
 * const value: string | number = await Select.prompt<string | number>({
 *   message: "Pick a color",
 *   options: [1, 2, "3", "4"],
 * });
 * ```
 *
 * None primitive option types:
 *
 * ```ts
 * import { Select } from "./mod.ts";
 *
 * const date: Date = await Select.prompt({
 *   message: "Pick a date",
 *   options: [
 *     {
 *       name: "Date 1",
 *       value: new Date(100000),
 *     },
 *     {
 *       name: "Date 2",
 *       value: new Date(200000),
 *     },
 *     {
 *       name: "Date 3",
 *       value: new Date(300000),
 *     },
 *   ],
 * });
 * ```
 *
 * Grouped options:
 *
 * ```ts
 * import { Select } from "./mod.ts";
 *
 * const value = await Select.prompt({
 *   message: "Select a value",
 *   options: [{
 *     name: "Group 1",
 *     options: ["foo", "bar", "baz"],
 *   }, {
 *     name: "Group 2",
 *     options: ["beep", "boop"],
 *   }],
 * });
 * ```
 */
export class Select extends GenericList {
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
        Object.defineProperty(this, "options", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "listIndex", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "listOffset", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        this.settings = this.getDefaultSettings(options);
        this.options = this.settings.options.slice();
        this.listIndex = this.getListIndex(this.settings.default);
        this.listOffset = this.getPageOffset(this.listIndex);
    }
    getDefaultSettings(options) {
        return {
            ...super.getDefaultSettings(options),
            options: this.mapOptions(options, options.options),
        };
    }
    /** Map string option values to options and set option defaults. */
    mapOptions(promptOptions, options) {
        return options.map((option) => isSelectOptionGroup(option)
            ? this.mapOptionGroup(promptOptions, option)
            : typeof option === "string" || typeof option === "number"
                ? this.mapOption(promptOptions, { value: option })
                : this.mapOption(promptOptions, option));
    }
    input() {
        return underline(brightBlue(this.inputValue));
    }
    async submit() {
        if (this.isBackButton(this.selectedOption) ||
            isOptionGroup(this.selectedOption)) {
            const info = isOptionGroup(this.selectedOption)
                ? ` To select a group use ${getFiguresByKeys(this.settings.keys.open ?? []).join(", ")}.`
                : "";
            this.setErrorMessage(`No option selected.${info}`);
            return;
        }
        await super.submit();
    }
    /** Get value of selected option. */
    getValue() {
        const option = this.options[this.listIndex];
        assertIsOption(option);
        return option.value;
    }
    /**
     * Validate input value.
     * @param value User input value.
     * @return True on success, false or error message on error.
     */
    validate(value) {
        return this.options.findIndex((option) => isOption(option) && equal(option.value, value)) !== -1;
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
        return this.settings.format?.(value) ??
            this.getOptionByValue(value)?.name ?? String(value);
    }
}
function assertIsOption(option) {
    if (!isOption(option)) {
        throw new Error("Expected an option but got an option group.");
    }
}
export function isSelectOptionGroup(option) {
    return isOptionGroup(option);
}
