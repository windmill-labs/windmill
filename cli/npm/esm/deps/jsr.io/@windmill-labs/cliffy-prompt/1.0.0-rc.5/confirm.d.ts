import { GenericSuggestions, type GenericSuggestionsKeys, type GenericSuggestionsOptions, type GenericSuggestionsSettings } from "./_generic_suggestions.js";
type UnsupportedOptions = "files" | "complete" | "suggestions" | "list" | "info";
/** Confirm prompt options. */
export interface ConfirmOptions extends Omit<GenericSuggestionsOptions<boolean, string>, UnsupportedOptions> {
    /** Keymap to assign key names to prompt actions. */
    keys?: ConfirmKeys;
    /** Change active label. Default is `Yes`. */
    active?: string;
    /** Change inactive label. Default is `No`. */
    inactive?: string;
}
/** Confirm prompt settings. */
interface ConfirmSettings extends GenericSuggestionsSettings<boolean, string> {
    active: string;
    inactive: string;
    keys?: ConfirmKeys;
}
/** Confirm prompt keymap. */
export type ConfirmKeys = GenericSuggestionsKeys;
/**
 * Confirm prompt representation.
 *
 * ```ts
 * import { Confirm } from "./mod.ts";
 *
 * const confirmed: boolean = await Confirm.prompt("Please confirm");
 * ```
 */
export declare class Confirm extends GenericSuggestions<boolean, string> {
    protected readonly settings: ConfirmSettings;
    /** Execute the prompt with provided options. */
    static prompt(options: string | ConfirmOptions): Promise<boolean>;
    /**
     * Inject prompt value. If called, the prompt doesn't prompt for an input and
     * returns immediately the injected value. Can be used for unit tests or pre
     * selections.
     *
     * @param value Input value.
     */
    static inject(value: string): void;
    constructor(options: string | ConfirmOptions);
    getDefaultSettings(options: ConfirmOptions): ConfirmSettings;
    protected defaults(): string;
    protected success(value: boolean): string | undefined;
    /** Get input input. */
    protected getValue(): string;
    /**
     * Validate input value.
     * @param value User input value.
     * @return True on success, false or error message on error.
     */
    protected validate(value: string): boolean | string;
    /**
     * Map input value to output value.
     * @param value Input value.
     * @return Output value.
     */
    protected transform(value: string): boolean | undefined;
    /**
     * Format output value.
     * @param value Output value.
     */
    protected format(value: boolean): string;
}
export {};
//# sourceMappingURL=confirm.d.ts.map