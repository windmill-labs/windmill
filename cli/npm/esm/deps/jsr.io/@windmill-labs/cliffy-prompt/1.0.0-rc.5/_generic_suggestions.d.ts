import type { KeyCode } from "../../cliffy-keycode/1.0.0-rc.5/mod.js";
import { GenericInput, type GenericInputKeys, type GenericInputPromptOptions, type GenericInputPromptSettings } from "./_generic_input.js";
/** Generic input prompt options. */
export interface GenericSuggestionsOptions<TValue, TRawValue> extends GenericInputPromptOptions<TValue, TRawValue> {
    /** Keymap to assign key names to prompt actions. */
    keys?: GenericSuggestionsKeys;
    /**
     * Prompt id. If set, the prompt value is stored in local storage and used for
     * auto suggestions the next time the prompt is used.
     */
    id?: string;
    /**
     * An array of suggestions or a callback function that returns an array of
     * suggestions.
     */
    suggestions?: Array<string | number> | SuggestionHandler;
    /** Callback function for auto-suggestion completion. */
    complete?: CompleteHandler;
    /**
     * Enable autosuggestions for files. Can be a boolean to enable all files or a
     * regular expression to include only specific files.
     */
    files?: boolean | RegExp;
    /** Show auto suggestions as a list. */
    list?: boolean;
    /** Display prompt info. */
    info?: boolean;
    /** Change list pointer. Default is `brightBlue("❯")`. */
    listPointer?: string;
    /** Limit max displayed rows per page. */
    maxRows?: number;
}
/** Generic input prompt settings. */
export interface GenericSuggestionsSettings<TValue, TRawValue> extends GenericInputPromptSettings<TValue, TRawValue> {
    keys?: GenericSuggestionsKeys;
    id?: string;
    suggestions?: Array<string | number> | SuggestionHandler;
    complete?: CompleteHandler;
    files?: boolean | RegExp;
    list?: boolean;
    info?: boolean;
    listPointer: string;
    maxRows: number;
}
/** Input keys options. */
export interface GenericSuggestionsKeys extends GenericInputKeys {
    /** Apply auto-suggestion keymap. Default is `["tab"]`. */
    complete?: string[];
    /** Select next option keymap. Default is `["up"]`. */
    next?: string[];
    /** Select previous option keymap. Default is `["down"]`. */
    previous?: string[];
    /** Select next page keymap. Default is `["pageup"]`. */
    nextPage?: string[];
    /** Select previous page keymap. Default is `["pagedown"]`. */
    previousPage?: string[];
}
/** Auto-suggestions handler. */
export type SuggestionHandler = (input: string) => Array<string | number> | Promise<Array<string | number>>;
/** Auto-suggestions complete handler. */
export type CompleteHandler = (input: string, suggestion?: string) => Promise<string> | string;
interface LocalStorage {
    getItem(key: string): string | null;
    removeItem(key: string): void;
    setItem(key: string, value: string): void;
}
/** Generic input prompt representation. */
export declare abstract class GenericSuggestions<TValue, TRawValue> extends GenericInput<TValue, TRawValue> {
    #private;
    protected abstract readonly settings: GenericSuggestionsSettings<TValue, TRawValue>;
    protected suggestionsIndex: number;
    protected suggestionsOffset: number;
    protected suggestions: Array<string | number>;
    getDefaultSettings(options: GenericSuggestionsOptions<TValue, TRawValue>): GenericSuggestionsSettings<TValue, TRawValue>;
    protected get localStorage(): LocalStorage | null;
    protected loadSuggestions(): Array<string | number>;
    protected saveSuggestions(...suggestions: Array<string | number>): void;
    protected render(): Promise<void>;
    protected match(): Promise<void>;
    protected input(): string;
    protected getSuggestion(): string;
    protected getUserSuggestions(input: string): Promise<Array<string | number>>;
    protected getFileSuggestions(input: string): Promise<Array<string | number>>;
    protected getSuggestions(): Promise<Array<string | number>>;
    protected body(): string | Promise<string>;
    protected getInfo(): string;
    protected getList(): string;
    /**
     * Render option.
     * @param value        Option.
     * @param isSelected  Set to true if option is selected.
     */
    protected getListItem(value: string | number, isSelected?: boolean): string;
    /** Get suggestions row height. */
    protected getListHeight(suggestions?: Array<string | number>): number;
    /**
     * Handle user input event.
     * @param event Key event.
     */
    protected handleEvent(event: KeyCode): Promise<void>;
    /** Delete char right. */
    protected deleteCharRight(): void;
    private setInputValue;
    protected complete(): Promise<string>;
    /** Select previous suggestion. */
    protected selectPreviousSuggestion(): void;
    /** Select next suggestion. */
    protected selectNextSuggestion(): void;
    /** Select previous suggestions page. */
    protected selectPreviousSuggestionsPage(): void;
    /** Select next suggestions page. */
    protected selectNextSuggestionsPage(): void;
}
export {};
//# sourceMappingURL=_generic_suggestions.d.ts.map