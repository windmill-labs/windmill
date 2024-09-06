import type { KeyCode } from "../../cliffy-keycode/1.0.0-rc.5/mod.js";
import { GenericPrompt, type GenericPromptKeys, type GenericPromptOptions, type GenericPromptSettings } from "./_generic_prompt.js";
/** Generic input prompt options. */
export interface GenericInputPromptOptions<TValue, TRawValue> extends GenericPromptOptions<TValue, TRawValue> {
    /** Keymap to assign key names to prompt actions. */
    keys?: GenericInputKeys;
}
/** Generic input prompt settings. */
export interface GenericInputPromptSettings<TValue, TRawValue> extends GenericPromptSettings<TValue, TRawValue> {
    keys?: GenericInputKeys;
}
/** Input keys options. */
export interface GenericInputKeys extends GenericPromptKeys {
    /** Cursor left keymap. Default is `["left"]`. */
    moveCursorLeft?: string[];
    /** Cursor right keymap. Default is `["right"]`. */
    moveCursorRight?: string[];
    /** Delete cursor left keymap. Default is `["backspace"]`. */
    deleteCharLeft?: string[];
    /** Delete cursor right keymap. Default is `["delete"]`. */
    deleteCharRight?: string[];
}
/** Generic input prompt representation. */
export declare abstract class GenericInput<TValue, TRawValue> extends GenericPrompt<TValue, TRawValue> {
    protected abstract readonly settings: GenericInputPromptSettings<TValue, TRawValue>;
    protected inputValue: string;
    protected inputIndex: number;
    getDefaultSettings(options: GenericInputPromptOptions<TValue, TRawValue>): GenericInputPromptSettings<TValue, TRawValue>;
    protected getCurrentInputValue(): string;
    protected message(): string;
    protected input(): string;
    protected highlight(value: string | number, color1?: (val: string) => string, color2?: (val: string) => string): string;
    /**
     * Handle user input event.
     * @param event Key event.
     */
    protected handleEvent(event: KeyCode): Promise<void>;
    /** Add character to current input. */
    protected addChar(char: string): void;
    /** Move prompt cursor left. */
    protected moveCursorLeft(): void;
    /** Move prompt cursor right. */
    protected moveCursorRight(): void;
    /** Delete char left. */
    protected deleteChar(): void;
    /** Delete char right. */
    protected deleteCharRight(): void;
}
//# sourceMappingURL=_generic_input.d.ts.map