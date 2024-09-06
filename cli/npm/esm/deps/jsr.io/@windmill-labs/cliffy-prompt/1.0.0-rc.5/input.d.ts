import { GenericSuggestions, type GenericSuggestionsKeys, type GenericSuggestionsOptions, type GenericSuggestionsSettings } from "./_generic_suggestions.js";
/** Input prompt options. */
export interface InputOptions extends GenericSuggestionsOptions<string, string> {
    /** Keymap to assign key names to prompt actions. */
    keys?: InputKeys;
    /** Set minimum allowed length of input value. */
    minLength?: number;
    /** Set maximum allowed length of input value. */
    maxLength?: number;
}
/** Input prompt settings. */
interface InputSettings extends GenericSuggestionsSettings<string, string> {
    minLength: number;
    maxLength: number;
    keys?: InputKeys;
}
/** Input prompt keymap. */
export type InputKeys = GenericSuggestionsKeys;
/**
 * Input prompt representation.
 *
 * ```ts
 * import { Input } from "./mod.ts";
 *
 * const confirmed: string = await Input.prompt("Enter your name");
 * ```
 */
export declare class Input extends GenericSuggestions<string, string> {
    protected readonly settings: InputSettings;
    /** Execute the prompt with provided options. */
    static prompt(options: string | InputOptions): Promise<string>;
    /**
     * Inject prompt value. If called, the prompt doesn't prompt for an input and
     * returns immediately the injected value. Can be used for unit tests or pre
     * selections.
     *
     * @param value Input value.
     */
    static inject(value: string): void;
    constructor(options: string | InputOptions);
    getDefaultSettings(options: InputOptions): InputSettings;
    protected success(value: string): string | undefined;
    /** Get input value. */
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
    protected transform(value: string): string | undefined;
    /**
     * Format output value.
     * @param value Output value.
     */
    protected format(value: string): string;
}
export {};
//# sourceMappingURL=input.d.ts.map