import type { KeyCode } from "../../cliffy-keycode/1.0.0-rc.5/mod.js";
import { GenericInput, type GenericInputKeys, type GenericInputPromptOptions, type GenericInputPromptSettings } from "./_generic_input.js";
import type { WidenType } from "./_utils.js";
type UnsupportedInputOptions = "suggestions" | "list";
/** Generic list prompt options. */
export interface GenericListOptions<TValue, TReturnValue, TRawValue> extends Omit<GenericInputPromptOptions<TReturnValue, TRawValue>, UnsupportedInputOptions> {
    options: Array<Extract<TValue, string | number> | Extract<WidenType<TValue>, string | number> | GenericListOption<TValue> | GenericListOptionGroup<TValue, GenericListOption<TValue>> | GenericListSeparatorOption>;
    /** Keymap to assign key names to prompt actions. */
    keys?: GenericListKeys;
    /** Change list pointer. Default is `brightBlue("❯")`. */
    listPointer?: string;
    /** Limit max displayed rows per page. */
    maxRows?: number;
    /** Change search label. Default is `brightBlue("🔎")`. */
    searchLabel?: string;
    /** Enable search. */
    search?: boolean;
    /** Display prompt info. */
    info?: boolean;
    /** Limit maximum amount of breadcrumb items. */
    maxBreadcrumbItems?: number;
    /** Change breadcrumb separator. Default is ` › `. */
    breadcrumbSeparator?: string;
    /** Change back pointer. Default is `❮`. */
    backPointer?: string;
    /** Change group pointer. Default is `❯`. */
    groupPointer?: string;
    /** Change group icon. Default is `📁`. */
    groupIcon?: string | boolean;
    /** Change opened group icon. Default is `📂`. */
    groupOpenIcon?: string | boolean;
    /** Format option value. */
    format?: (value: TValue) => string;
}
/** Generic list prompt settings. */
export interface GenericListSettings<TValue, TReturnValue, TRawValue, TOption extends GenericListOptionSettings<TValue>, TGroup extends GenericListOptionGroupSettings<TValue, TOption>> extends GenericInputPromptSettings<TReturnValue, TRawValue> {
    options: Array<TOption | TGroup>;
    keys: GenericListKeys;
    listPointer: string;
    maxRows: number;
    searchLabel: string;
    search?: boolean;
    info?: boolean;
    maxBreadcrumbItems: number;
    breadcrumbSeparator: string;
    backPointer: string;
    groupPointer: string;
    groupIcon: string | false;
    groupOpenIcon: string | false;
    format?: (value: TValue) => string;
}
/** Generic list separator option options. */
export interface GenericListSeparatorOption {
    /** The separator label. */
    name: string;
}
/** Generic list option options. */
export interface GenericListOption<TValue> {
    /** The option value. */
    value: TValue;
    /** The option label. */
    name?: string;
    /** Disable option. Disabled options are displayed but cannot be selected. */
    disabled?: boolean;
}
/** Generic list option group options. */
export interface GenericListOptionGroup<TValue, TOption extends GenericListOption<TValue>> {
    /** The option label. */
    name: string;
    /** An array of child options. */
    options: Array<Extract<TValue, string | number> | Extract<WidenType<TValue>, string | number> | TOption | this | GenericListSeparatorOption>;
    /** Disable option. Disabled options are displayed but cannot be selected. */
    disabled?: boolean;
}
/** Generic list option settings. */
export interface GenericListOptionSettings<TValue> extends GenericListOption<TValue> {
    name: string;
    disabled: boolean;
    indentLevel: number;
}
/** Generic list option group settings. */
export interface GenericListOptionGroupSettings<TValue, TOption extends GenericListOptionSettings<TValue>> extends GenericListOptionGroup<TValue, TOption> {
    disabled: boolean;
    indentLevel: number;
    options: Array<TOption | this>;
}
/** GenericList key options. */
export interface GenericListKeys extends GenericInputKeys {
    /** Select next option keymap. Default is `["down", "d", "n", "2"]`. */
    next?: string[];
    /** Select previous option keymap. Default is `["up", "u", "p", "8"]`. */
    previous?: string[];
    /** Select next page keymap. Default is `["pagedown", "right"]`. */
    nextPage?: string[];
    /** Select previous page keymap. Default is `["pageup", "left"]`. */
    previousPage?: string[];
    /** Select next option keymap. Default is `["right", "enter", "return"]`. */
    open?: string[];
    /** Select next option keymap. Default is `["left", "escape", "enter", "return"]`. */
    back?: string[];
}
/** Generic list prompt representation. */
export declare abstract class GenericList<TValue, TReturnValue, TRawValue, TOption extends GenericListOptionSettings<TValue>, TGroup extends GenericListOptionGroupSettings<TValue, TOption>> extends GenericInput<TReturnValue, TRawValue> {
    protected abstract readonly settings: GenericListSettings<TValue, TReturnValue, TRawValue, TOption, TGroup>;
    protected abstract options: Array<TOption | TGroup>;
    protected abstract listIndex: number;
    protected abstract listOffset: number;
    protected parentOptions: Array<TGroup>;
    protected get selectedOption(): TOption | TGroup | undefined;
    /**
     * Create list separator.
     *
     * @param label Separator label.
     */
    static separator(label?: string): GenericListSeparatorOption;
    getDefaultSettings({ groupIcon, groupOpenIcon, ...options }: GenericListOptions<TValue, TReturnValue, TRawValue>): GenericListSettings<TValue, TReturnValue, TRawValue, TOption, TGroup>;
    protected abstract mapOptions(promptOptions: GenericListOptions<TValue, TReturnValue, TRawValue>, options: Array<Extract<TValue, string | number> | Extract<WidenType<TValue>, string | number> | GenericListOption<TValue> | GenericListOptionGroup<TValue, GenericListOption<TValue>> | GenericListSeparatorOption>): Array<TOption | TGroup>;
    protected mapOption(options: GenericListOptions<TValue, TReturnValue, TRawValue>, option: GenericListOption<TValue> | GenericListSeparatorOption): GenericListOptionSettings<TValue>;
    protected mapOptionGroup(options: GenericListOptions<TValue, TReturnValue, TRawValue>, option: GenericListOptionGroup<TValue, GenericListOption<TValue>>, recursive?: boolean): GenericListOptionGroupSettings<TValue, GenericListOptionSettings<TValue>>;
    protected match(): void;
    protected setOptions(options: Array<TOption | TGroup>): void;
    protected getCurrentOptions(): Array<TOption | TGroup>;
    protected getParentOption(index?: number): TGroup | undefined;
    protected submitBackButton(): void;
    protected submitGroupOption(selectedOption: TGroup): void;
    protected isBackButton(option: TOption | TGroup | undefined): boolean;
    protected hasParent(): boolean;
    protected isSearching(): boolean;
    protected message(): string;
    /** Render options. */
    protected body(): string | Promise<string>;
    protected getInfo(): string;
    /** Render options list. */
    protected getList(): string;
    /**
     * Render option.
     * @param option        Option.
     * @param isSelected  Set to true if option is selected.
     */
    protected getListItem(option: TOption | TGroup, isSelected?: boolean): string;
    protected getListItemIndent(option: TOption | TGroup): string;
    protected getListItemPointer(option: TOption | TGroup, isSelected?: boolean): string;
    protected getListItemIcon(option: TOption | TGroup): string;
    protected getListItemLabel(option: TOption | TGroup, isSelected?: boolean): string;
    protected getBreadCrumb(): string;
    /** Get options row height. */
    protected getListHeight(): number;
    protected getListIndex(value?: TValue): number;
    protected getPageOffset(index: number): number;
    /**
     * Find option by value.
     * @param value Value of the option.
     */
    protected getOptionByValue(value: TValue): TOption | undefined;
    /** Read user input. */
    protected read(): Promise<boolean>;
    protected selectSearch(): void;
    protected isSearchSelected(): boolean;
    /**
     * Handle user input event.
     * @param event Key event.
     */
    protected handleEvent(event: KeyCode): Promise<void>;
    protected submit(): Promise<void>;
    protected moveCursorLeft(): void;
    protected moveCursorRight(): void;
    protected deleteChar(): void;
    protected deleteCharRight(): void;
    protected addChar(char: string): void;
    /** Select previous option. */
    protected selectPrevious(loop?: boolean): void;
    /** Select next option. */
    protected selectNext(loop?: boolean): void;
    /** Select previous page. */
    protected selectPreviousPage(): void;
    /** Select next page. */
    protected selectNextPage(): void;
}
export declare function isOption<TValue, TOption extends GenericListOption<TValue>>(option: TOption | GenericListOptionGroup<TValue, GenericListOption<TValue>> | GenericListSeparatorOption | undefined): option is TOption;
export declare function isOptionGroup<TValue, TGroup extends GenericListOptionGroup<TValue, GenericListOption<TValue>>>(option: TGroup | TValue | GenericListOption<TValue> | GenericListSeparatorOption | undefined): option is TGroup;
/**
 * GenericList options type.
 *
 * @deprecated Use `Array<string | GenericListOption>` instead.
 */
export type GenericListValueOptions = Array<string | GenericListOption<string>>;
/** @deprecated Use `Array<GenericListOptionSettings>` instead. */
export type GenericListValueSettings = Array<GenericListOptionSettings<string>>;
export {};
//# sourceMappingURL=_generic_list.d.ts.map