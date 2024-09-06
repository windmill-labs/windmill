import { GenericInput, type GenericInputKeys, type GenericInputPromptOptions, type GenericInputPromptSettings } from "./_generic_input.js";
/** Secret prompt options. */
export interface SecretOptions extends GenericInputPromptOptions<string, string> {
    keys?: SecretKeys;
    /** Change prompt label. Default is `Secret`. */
    label?: string;
    /**
     * If enabled, the input value is hidden, otherwise each character is replaced
     * with a `*`.
     */
    hidden?: boolean;
    /** Limit minimum allowed length of the secret. */
    minLength?: number;
    /** Limit maximum allowed length of the secret. */
    maxLength?: number;
}
/** Secret prompt settings. */
interface SecretSettings extends GenericInputPromptSettings<string, string> {
    label: string;
    hidden: boolean;
    minLength: number;
    maxLength: number;
    keys?: SecretKeys;
}
/** Secret prompt keymap. */
export type SecretKeys = GenericInputKeys;
/**
 * Secret prompt representation.
 *
 * ```ts
 * import { Secret } from "./mod.ts";
 *
 * const password: string = await Secret.prompt("Enter your password");
 * ```
 */
export declare class Secret extends GenericInput<string, string> {
    protected readonly settings: SecretSettings;
    /** Execute the prompt with provided message or options. */
    static prompt(options: string | SecretOptions): Promise<string>;
    /**
     * Inject prompt value. If called, the prompt doesn't prompt for an input and
     * returns immediately the injected value. Can be used for unit tests or pre
     * selections.
     *
     * @param value Input value.
     */
    static inject(value: string): void;
    constructor(options: string | SecretOptions);
    getDefaultSettings(options: SecretOptions): SecretSettings;
    protected input(): string;
    /** Read user input. */
    protected read(): Promise<boolean>;
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
    /** Get input input. */
    protected getValue(): string;
}
export {};
//# sourceMappingURL=secret.d.ts.map