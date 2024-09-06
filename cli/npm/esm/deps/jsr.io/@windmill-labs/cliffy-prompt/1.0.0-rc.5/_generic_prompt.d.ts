import * as dntShim from "../../../../../_dnt.shims.js";
import type { Cursor } from "../../cliffy-ansi/1.0.0-rc.5/cursor_position.js";
import { type Tty } from "../../cliffy-ansi/1.0.0-rc.5/tty.js";
import { type KeyCode } from "../../cliffy-keycode/1.0.0-rc.5/mod.js";
import type { Reader, WriterSync } from "../../../@std/io/0.224.7/types.js";
/** Static generic prompt interface. */
export interface StaticGenericPrompt<TValue, TOptions> {
    /**
     * Inject prompt value. If called, the prompt doesn't prompt for an input and
     * returns immediately the injected value. Can be used for unit tests or pre
     * selections.
     *
     * @param value The injected value.
     */
    inject?(value: TValue): void;
    /**
     * Execute the prompt with the given options.
     *
     * @param options Prompt options.
     */
    prompt(options: TOptions): Promise<TValue>;
}
export type InferPromptOptions<TPrompt extends GenericPrompt<any, any>> = Parameters<TPrompt["getDefaultSettings"]>[0];
export type InferPromptValue<TPrompt extends GenericPrompt<any, any>> = Awaited<ReturnType<TPrompt["prompt"]>>;
/** Generic prompt options. */
export interface GenericPromptOptions<TValue, TRawValue> {
    /** The prompt message. */
    message: string;
    /** The default value of the prompt. */
    default?: TValue;
    /** Don't show the default value after the prompt message. */
    hideDefault?: boolean;
    /** Validate prompt value. */
    validate?: (value: TRawValue) => ValidateResult;
    /** Transform input value to output value. */
    transform?: (value: TRawValue) => TValue | undefined;
    /** Show a hint below the prompt. */
    hint?: string;
    /** Change the prompt pointer. Default is `brightBlue("›")`. */
    pointer?: string;
    /** Indent the prompt. */
    indent?: string;
    /** Keymap to assign key names to prompt actions. */
    keys?: GenericPromptKeys;
    /** Enable cbreak mode. For more information see [Deno.SetRawOptions](https://deno.land/api@v1.42.1?s=Deno.SetRawOptions). */
    cbreak?: boolean;
    /** Change the prompt prefix. Default is: `yellow("? ")`. */
    prefix?: string;
    /** Change the prompt input stream. */
    reader?: Reader & {
        setRaw(mode: boolean, options?: dntShim.Deno.SetRawOptions): void;
        isTerminal(): boolean;
    };
    /** Change the prompt output stream. */
    writer?: WriterSync;
}
/** Generic prompt settings. */
export interface GenericPromptSettings<TValue, TRawValue> extends GenericPromptOptions<TValue, TRawValue> {
    pointer: string;
    indent: string;
    prefix: string;
    cbreak: boolean;
    tty: Tty;
    reader: Reader & {
        setRaw(mode: boolean, options?: dntShim.Deno.SetRawOptions): void;
        isTerminal(): boolean;
    };
    writer: WriterSync;
}
/** Prompt validation return tape. */
export type ValidateResult = string | boolean | Promise<string | boolean>;
/** Generic prompt keymap. */
export interface GenericPromptKeys {
    /** Submit keymap. Default is `["enter", "return"]`. */
    submit?: Array<string>;
}
/** Generic prompt representation. */
export declare abstract class GenericPrompt<TValue, TRawValue> {
    #private;
    protected static injectedValue: unknown | undefined;
    protected abstract readonly settings: GenericPromptSettings<TValue, TRawValue>;
    protected readonly cursor: Cursor;
    /**
     * Inject prompt value. If called, the prompt doesn't prompt for an input and
     * returns immediately the injected value. Can be used for unit tests or pre
     * selections.
     *
     * @param value Input value.
     */
    static inject(value: unknown): void;
    getDefaultSettings(options: GenericPromptOptions<TValue, TRawValue>): GenericPromptSettings<TValue, TRawValue>;
    /** Execute the prompt. */
    prompt(): Promise<TValue>;
    /** Clear prompt output. */
    protected clear(): void;
    /** Render prompt. */
    protected render(): Promise<void>;
    /** Read user input from stdin, handle events and validate user input. */
    protected read(): Promise<boolean>;
    protected submit(): Promise<void>;
    protected message(): string;
    protected defaults(): string;
    /** Get prompt success message. */
    protected success(value: TValue): string | undefined;
    protected body?(): string | undefined | Promise<string | undefined>;
    protected footer(): string | undefined;
    protected error(): string | undefined;
    protected hint(): string | undefined;
    protected setErrorMessage(message: string): void;
    /**
     * Handle user input event.
     * @param event Key event.
     */
    protected handleEvent(event: KeyCode): Promise<void>;
    /**
     * Map input value to output value.
     * @param value Input value.
     * @return Output value.
     */
    protected abstract transform(value: TRawValue): TValue | undefined;
    /**
     * Validate input value.
     * @param value User input value.
     * @return True on success, false or error message on error.
     */
    protected abstract validate(value: TRawValue): ValidateResult;
    /**
     * Format output value.
     * @param value Output value.
     */
    protected abstract format(value: TValue): string;
    /** Get input value. */
    protected abstract getValue(): TRawValue;
    /**
     * Check if key event has given name or sequence.
     * @param keys  Key map.
     * @param name  Key name.
     * @param event Key event.
     */
    protected isKey<TKey extends unknown, TName extends keyof TKey>(keys: TKey | undefined, name: TName, event: KeyCode): boolean;
}
//# sourceMappingURL=_generic_prompt.d.ts.map